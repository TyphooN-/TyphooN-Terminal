// ── Round 15 tests ─────────────────────────────────────────────

#[test]
fn val_snapshot_roundtrip() {
    let c = Connection::open_in_memory().unwrap();
    create_research_tables_v15(&c).unwrap();
    let snap = ValueSnapshot {
        symbol: "VAL1".to_string(),
        as_of: "2026-04-14".to_string(),
        sector: "Technology".to_string(),
        peers_considered: 9,
        value_label: "VALUE".to_string(),
        ..Default::default()
    };
    upsert_val(&c, "val1", &snap).unwrap();
    let got = get_val(&c, "VAL1").unwrap().unwrap();
    assert_eq!(got.value_label, "VALUE");
    assert_eq!(got.peers_considered, 9);
}

#[test]
fn qual_snapshot_roundtrip() {
    let c = Connection::open_in_memory().unwrap();
    create_research_tables_v15(&c).unwrap();
    let snap = QualitySnapshot {
        symbol: "QL1".to_string(),
        as_of: "2026-04-14".to_string(),
        quality_label: "HIGH_QUALITY".to_string(),
        composite_score: 82.0,
        ..Default::default()
    };
    upsert_qual(&c, "ql1", &snap).unwrap();
    let got = get_qual(&c, "QL1").unwrap().unwrap();
    assert_eq!(got.quality_label, "HIGH_QUALITY");
}

#[test]
fn risk_snapshot_roundtrip() {
    let c = Connection::open_in_memory().unwrap();
    create_research_tables_v15(&c).unwrap();
    let snap = RiskSnapshot {
        symbol: "RK1".to_string(),
        as_of: "2026-04-14".to_string(),
        risk_label: "MODERATE".to_string(),
        composite_score: 42.0,
        ..Default::default()
    };
    upsert_risk(&c, "rk1", &snap).unwrap();
    let got = get_risk(&c, "RK1").unwrap().unwrap();
    assert_eq!(got.risk_label, "MODERATE");
}

#[test]
fn insstrk_snapshot_roundtrip() {
    let c = Connection::open_in_memory().unwrap();
    create_research_tables_v15(&c).unwrap();
    let snap = InsiderStreakSnapshot {
        symbol: "INS1".to_string(),
        as_of: "2026-04-14".to_string(),
        window_days: 180,
        unique_insiders: 4,
        streak_label: "ACCUMULATION".to_string(),
        ..Default::default()
    };
    upsert_insstrk(&c, "ins1", &snap).unwrap();
    let got = get_insstrk(&c, "INS1").unwrap().unwrap();
    assert_eq!(got.streak_label, "ACCUMULATION");
}

#[test]
fn covg_snapshot_roundtrip() {
    let c = Connection::open_in_memory().unwrap();
    create_research_tables_v15(&c).unwrap();
    let snap = CoverageSnapshot {
        symbol: "CVG1".to_string(),
        as_of: "2026-04-14".to_string(),
        num_analysts: 18,
        coverage_label: "STABLE".to_string(),
        ..Default::default()
    };
    upsert_covg(&c, "cvg1", &snap).unwrap();
    let got = get_covg(&c, "CVG1").unwrap().unwrap();
    assert_eq!(got.coverage_label, "STABLE");
}

#[test]
fn compute_val_value_label() {
    use crate::core::fundamentals::Fundamentals;
    let subject = Fundamentals {
        symbol: "SUB".to_string(),
        pe_ratio: Some(10.0),
        forward_pe: Some(9.0),
        price_to_book: Some(1.0),
        price_to_sales: Some(1.0),
        ev_to_ebitda: Some(6.0),
        ..Default::default()
    };
    let peers: Vec<Fundamentals> = (0..5)
        .map(|i| Fundamentals {
            symbol: format!("P{}", i),
            pe_ratio: Some(20.0),
            forward_pe: Some(18.0),
            price_to_book: Some(3.0),
            price_to_sales: Some(3.0),
            ev_to_ebitda: Some(14.0),
            ..Default::default()
        })
        .collect();
    let fcfy = FcfYieldSnapshot {
        ttm_fcf_yield_pct: 8.0,
        ..Default::default()
    };
    let peer_fcfy = vec![4.0, 4.0, 4.0, 4.0, 4.0];
    let snap = compute_val_snapshot(
        "SUB",
        "2026-04-14",
        "Technology",
        Some(&subject),
        &peers,
        Some(&fcfy),
        &peer_fcfy,
    );
    assert!(matches!(snap.value_label.as_str(), "DEEP_VALUE" | "VALUE"));
    assert!(snap.composite_score >= 65.0);
    assert_eq!(snap.inputs_available, 6);
}

#[test]
fn compute_val_no_data() {
    let snap = compute_val_snapshot("SUB", "2026-04-14", "Technology", None, &[], None, &[]);
    assert_eq!(snap.value_label, "NO_DATA");
}

#[test]
fn compute_qual_high_quality() {
    let pt = PiotroskiSnapshot {
        f_score: 8,
        strength_label: "STRONG".to_string(),
        ..Default::default()
    };
    let mg = MarginsSnapshot {
        latest_operating_margin_pct: 28.0,
        overall_trend_label: "EXPANDING".to_string(),
        quality_label: "HIGH".to_string(),
        ..Default::default()
    };
    let ac = AccrualsSnapshot {
        ttm_cash_conversion_pct: 115.0,
        trend_label: "IMPROVING".to_string(),
        ..Default::default()
    };
    let lv = LeverageSnapshot {
        total_debt: 100.0,
        ebitda_ttm: 200.0,
        solvency_summary: "HEALTHY".to_string(),
        ..Default::default()
    };
    let snap = compute_qual_snapshot(
        "QQ",
        "2026-04-14",
        Some(&pt),
        Some(&mg),
        Some(&ac),
        Some(&lv),
    );
    assert!(matches!(
        snap.quality_label.as_str(),
        "HIGH_QUALITY" | "QUALITY"
    ));
    assert_eq!(snap.inputs_available, 4);
    assert!(snap.composite_score >= 70.0);
}

#[test]
fn compute_qual_no_inputs() {
    let snap = compute_qual_snapshot("QQ", "2026-04-14", None, None, None, None);
    assert_eq!(snap.quality_label, "NO_DATA");
}

#[test]
fn compute_risk_distressed() {
    let altz = AltmanZSnapshot {
        z_score: 1.2,
        zone: "DISTRESS".to_string(),
        ..Default::default()
    };
    let snap = compute_risk_snapshot("RK", "2026-04-14", None, None, None, None, Some(&altz));
    assert_eq!(snap.risk_label, "DISTRESSED");
}

#[test]
fn compute_risk_low() {
    let vole = OhlcVolSnapshot {
        preferred_estimate_pct: 12.0,
        preferred_label: "Yang-Zhang".to_string(),
        ..Default::default()
    };
    let beta = BetaSnapshot {
        windows: vec![BetaWindow {
            window_label: "1Y".to_string(),
            beta: 1.05,
            n_observations: 252,
            ..Default::default()
        }],
        ..Default::default()
    };
    let liq = LiquiditySnapshot {
        liquidity_tier: "DEEP".to_string(),
        ..Default::default()
    };
    let altz = AltmanZSnapshot {
        z_score: 4.5,
        zone: "SAFE".to_string(),
        ..Default::default()
    };
    let snap = compute_risk_snapshot(
        "RK",
        "2026-04-14",
        Some(&vole),
        Some(&beta),
        Some(&liq),
        None,
        Some(&altz),
    );
    assert_eq!(snap.risk_label, "LOW_RISK");
    assert_eq!(snap.inputs_available, 4);
}

#[test]
fn compute_risk_no_inputs() {
    let snap = compute_risk_snapshot("RK", "2026-04-14", None, None, None, None, None);
    assert_eq!(snap.risk_label, "NO_DATA");
}

#[test]
fn compute_insstrk_accumulation() {
    let trades = vec![
        InsiderTrade {
            transaction_date: "2026-03-01".to_string(),
            reporting_name: "Alice CFO".to_string(),
            transaction_type: "P-Purchase".to_string(),
            acquisition_disposition: "A".to_string(),
            shares: 1_000.0,
            value_usd: 50_000.0,
            ..Default::default()
        },
        InsiderTrade {
            transaction_date: "2026-03-10".to_string(),
            reporting_name: "Alice CFO".to_string(),
            transaction_type: "P-Purchase".to_string(),
            acquisition_disposition: "A".to_string(),
            shares: 1_000.0,
            value_usd: 55_000.0,
            ..Default::default()
        },
        InsiderTrade {
            transaction_date: "2026-03-05".to_string(),
            reporting_name: "Bob CEO".to_string(),
            transaction_type: "P-Purchase".to_string(),
            acquisition_disposition: "A".to_string(),
            shares: 500.0,
            value_usd: 25_000.0,
            ..Default::default()
        },
        InsiderTrade {
            transaction_date: "2026-03-20".to_string(),
            reporting_name: "Bob CEO".to_string(),
            transaction_type: "P-Purchase".to_string(),
            acquisition_disposition: "A".to_string(),
            shares: 500.0,
            value_usd: 27_000.0,
            ..Default::default()
        },
    ];
    let snap = compute_insstrk_snapshot("INS", "2026-04-14", &trades, 180);
    assert_eq!(snap.unique_insiders, 2);
    assert!(matches!(
        snap.streak_label.as_str(),
        "ACCUMULATION" | "STRONG_ACCUMULATION" | "MIXED"
    ));
    assert!(snap.buy_streak_count >= 2);
}

#[test]
fn compute_insstrk_none() {
    let snap = compute_insstrk_snapshot("INS", "2026-04-14", &[], 180);
    assert_eq!(snap.streak_label, "NONE");
}

#[test]
fn compute_covg_stable() {
    let pt = PriceTarget {
        symbol: "CVG".to_string(),
        target_mean: 150.0,
        target_low: 120.0,
        target_high: 180.0,
        num_analysts: 18,
        ..Default::default()
    };
    let recs = vec![AnalystRecommendation {
        period: "2026-04-01".to_string(),
        strong_buy: 6,
        buy: 8,
        hold: 3,
        sell: 1,
        strong_sell: 0,
    }];
    let updm = UpdmSnapshot {
        total_actions: 10,
        upgrades_90d: 5,
        downgrades_90d: 3,
        net_90d: 2,
        ..Default::default()
    };
    let snap = compute_covg_snapshot("CVG", "2026-04-14", Some(&pt), &recs, Some(&updm));
    assert!(matches!(
        snap.coverage_label.as_str(),
        "STABLE" | "EXPANDING"
    ));
    assert_eq!(snap.inputs_available, 3);
}

#[test]
fn compute_covg_none() {
    let snap = compute_covg_snapshot("CVG", "2026-04-14", None, &[], None);
    assert_eq!(snap.coverage_label, "NONE");
}

// ── Round 16 tests ─────────────────────────────────────────────

#[test]
fn vrk_snapshot_roundtrip() {
    let c = Connection::open_in_memory().unwrap();
    create_research_tables_v16(&c).unwrap();
    let snap = ValueRankSnapshot {
        symbol: "VRK1".into(),
        as_of: "2026-04-15".into(),
        sector: "Tech".into(),
        rank_label: "TOP_QUARTILE".into(),
        percentile_rank: 78.5,
        ..Default::default()
    };
    upsert_vrk(&c, "vrk1", &snap).unwrap();
    let got = get_vrk(&c, "VRK1").unwrap().unwrap();
    assert_eq!(got.rank_label, "TOP_QUARTILE");
    assert!((got.percentile_rank - 78.5).abs() < 1e-6);
}

#[test]
fn qrk_snapshot_roundtrip() {
    let c = Connection::open_in_memory().unwrap();
    create_research_tables_v16(&c).unwrap();
    let snap = QualityRankSnapshot {
        symbol: "QRK1".into(),
        as_of: "2026-04-15".into(),
        sector: "Healthcare".into(),
        rank_label: "ABOVE_MEDIAN".into(),
        ..Default::default()
    };
    upsert_qrk(&c, "qrk1", &snap).unwrap();
    let got = get_qrk(&c, "QRK1").unwrap().unwrap();
    assert_eq!(got.rank_label, "ABOVE_MEDIAN");
}

#[test]
fn rrk_snapshot_roundtrip() {
    let c = Connection::open_in_memory().unwrap();
    create_research_tables_v16(&c).unwrap();
    let snap = RiskRankSnapshot {
        symbol: "RRK1".into(),
        as_of: "2026-04-15".into(),
        sector: "Energy".into(),
        rank_label: "SAFEST_QUARTILE".into(),
        ..Default::default()
    };
    upsert_rrk(&c, "rrk1", &snap).unwrap();
    let got = get_rrk(&c, "RRK1").unwrap().unwrap();
    assert_eq!(got.rank_label, "SAFEST_QUARTILE");
}

#[test]
fn relepsgr_snapshot_roundtrip() {
    let c = Connection::open_in_memory().unwrap();
    create_research_tables_v16(&c).unwrap();
    let snap = RelativeEpsGrowthSnapshot {
        symbol: "RELEPS1".into(),
        as_of: "2026-04-15".into(),
        sector: "Tech".into(),
        symbol_cagr_pct: 22.0,
        sector_median_cagr_pct: 12.0,
        relative_label: "ABOVE".into(),
        ..Default::default()
    };
    upsert_relepsgr(&c, "releps1", &snap).unwrap();
    let got = get_relepsgr(&c, "RELEPS1").unwrap().unwrap();
    assert_eq!(got.relative_label, "ABOVE");
}

#[test]
fn pead_snapshot_roundtrip() {
    let c = Connection::open_in_memory().unwrap();
    create_research_tables_v16(&c).unwrap();
    let snap = PeadSnapshot {
        symbol: "PEAD1".into(),
        as_of: "2026-04-15".into(),
        num_events: 8,
        events_used: 6,
        avg_drift_5d_pct: 3.2,
        drift_direction_label: "DRIFT_UP".into(),
        ..Default::default()
    };
    upsert_pead(&c, "pead1", &snap).unwrap();
    let got = get_pead(&c, "PEAD1").unwrap().unwrap();
    assert_eq!(got.drift_direction_label, "DRIFT_UP");
}

fn mk_val_snap(sym: &str, sector: &str, score: f64) -> ValueSnapshot {
    ValueSnapshot {
        symbol: sym.to_string(),
        as_of: "2026-04-15".into(),
        sector: sector.to_string(),
        composite_score: score,
        value_label: if score >= 65.0 {
            "VALUE".into()
        } else {
            "FAIR".into()
        },
        inputs_available: 6,
        ..Default::default()
    }
}

#[test]
fn compute_vrk_top_decile() {
    let subj = mk_val_snap("SUB", "Tech", 92.0);
    let peer_vec: Vec<ValueSnapshot> = (0..9)
        .map(|i| mk_val_snap(&format!("P{}", i), "Tech", 30.0 + i as f64 * 5.0))
        .collect();
    let peers: Vec<&ValueSnapshot> = peer_vec.iter().collect();
    let snap = compute_vrk_snapshot("SUB", "2026-04-15", Some(&subj), &peers);
    assert_eq!(snap.rank_label, "TOP_DECILE");
    assert!(snap.percentile_rank >= 90.0);
    assert_eq!(snap.rank_position, 1);
    assert_eq!(snap.peers_considered, 9);
}

#[test]
fn compute_vrk_insufficient_peers() {
    let subj = mk_val_snap("SUB", "Tech", 70.0);
    let peer_vec = vec![
        mk_val_snap("P1", "Tech", 50.0),
        mk_val_snap("P2", "Tech", 60.0),
    ];
    let peers: Vec<&ValueSnapshot> = peer_vec.iter().collect();
    let snap = compute_vrk_snapshot("SUB", "2026-04-15", Some(&subj), &peers);
    assert_eq!(snap.rank_label, "NO_DATA");
}

fn mk_qual_snap(sym: &str, score: f64) -> QualitySnapshot {
    QualitySnapshot {
        symbol: sym.to_string(),
        as_of: "2026-04-15".into(),
        composite_score: score,
        quality_label: if score >= 65.0 {
            "QUALITY".into()
        } else {
            "AVERAGE".into()
        },
        inputs_available: 4,
        ..Default::default()
    }
}

#[test]
fn compute_qrk_above_median() {
    let subj = mk_qual_snap("SUB", 72.0);
    let peer_vec: Vec<QualitySnapshot> = (0..9)
        .map(|i| mk_qual_snap(&format!("P{}", i), 40.0 + i as f64 * 4.0))
        .collect();
    let peers: Vec<&QualitySnapshot> = peer_vec.iter().collect();
    let snap = compute_qrk_snapshot("SUB", "2026-04-15", "Tech", Some(&subj), &peers);
    assert!(matches!(
        snap.rank_label.as_str(),
        "ABOVE_MEDIAN" | "TOP_QUARTILE" | "TOP_DECILE"
    ));
    assert!(snap.percentile_rank >= 50.0);
}

#[test]
fn compute_qrk_no_data() {
    let snap = compute_qrk_snapshot("SUB", "2026-04-15", "Tech", None, &[]);
    assert_eq!(snap.rank_label, "NO_DATA");
}

fn mk_risk_snap(sym: &str, composite: f64) -> RiskSnapshot {
    RiskSnapshot {
        symbol: sym.to_string(),
        as_of: "2026-04-15".into(),
        composite_score: composite,
        risk_label: if composite >= 75.0 {
            "HIGH_RISK".into()
        } else {
            "MODERATE".into()
        },
        inputs_available: 5,
        ..Default::default()
    }
}

#[test]
fn compute_rrk_safest() {
    // Subject has the LOWEST risk composite in the cohort → SAFEST_DECILE.
    let subj = mk_risk_snap("SUB", 10.0);
    let peer_vec: Vec<RiskSnapshot> = (0..9)
        .map(|i| mk_risk_snap(&format!("P{}", i), 40.0 + i as f64 * 5.0))
        .collect();
    let peers: Vec<&RiskSnapshot> = peer_vec.iter().collect();
    let snap = compute_rrk_snapshot("SUB", "2026-04-15", "Tech", Some(&subj), &peers);
    assert_eq!(snap.rank_label, "SAFEST_DECILE");
    assert!(snap.percentile_rank >= 90.0);
    assert_eq!(snap.rank_position, 1);
}

#[test]
fn compute_rrk_riskiest() {
    // Subject has the HIGHEST risk composite → RISKIEST_DECILE.
    let subj = mk_risk_snap("SUB", 95.0);
    let peer_vec: Vec<RiskSnapshot> = (0..9)
        .map(|i| mk_risk_snap(&format!("P{}", i), 20.0 + i as f64 * 5.0))
        .collect();
    let peers: Vec<&RiskSnapshot> = peer_vec.iter().collect();
    let snap = compute_rrk_snapshot("SUB", "2026-04-15", "Tech", Some(&subj), &peers);
    assert_eq!(snap.rank_label, "RISKIEST_DECILE");
    assert!(snap.percentile_rank < 10.0);
}

fn mk_financials_with_eps(annual_eps_newest_first: &[f64]) -> FinancialStatements {
    let income: Vec<IncomeStatement> = annual_eps_newest_first
        .iter()
        .enumerate()
        .map(|(i, &eps)| IncomeStatement {
            date: format!("202{}-12-31", 6 - i),
            period: "FY".into(),
            eps,
            eps_diluted: eps,
            ..Default::default()
        })
        .collect();
    FinancialStatements {
        income_annual: income,
        ..Default::default()
    }
}

#[test]
fn compute_relepsgr_above() {
    // Subject EPS: 8 → 4 (newest to oldest), 3-year CAGR ≈ 26 %.
    let subj = mk_financials_with_eps(&[8.0, 7.0, 5.5, 4.0]);
    // Peers: flat ~12 % CAGR (2.0 → 1.42)
    let peer_vec: Vec<(String, FinancialStatements)> = (0..5)
        .map(|i| {
            (
                format!("P{}", i),
                mk_financials_with_eps(&[2.0, 1.8, 1.6, 1.42]),
            )
        })
        .collect();
    let snap = compute_relepsgr_snapshot("SUB", "2026-04-15", "Tech", Some(&subj), &peer_vec);
    assert!(matches!(
        snap.relative_label.as_str(),
        "ABOVE" | "FAR_ABOVE"
    ));
    assert!(snap.symbol_cagr_pct > snap.sector_median_cagr_pct);
    assert_eq!(snap.years_used, 3);
}

#[test]
fn compute_relepsgr_insufficient() {
    let subj = mk_financials_with_eps(&[5.0, 4.0]); // only 2 rows
    let snap = compute_relepsgr_snapshot("SUB", "2026-04-15", "Tech", Some(&subj), &[]);
    assert_eq!(snap.relative_label, "NO_DATA");
}

#[test]
fn compute_pead_drift_up() {
    // Build 15 newest-first HP bars with a steady 1%/day advance.
    let mut bars: Vec<HistoricalPriceRow> = (0..15)
        .map(|i| HistoricalPriceRow {
            date: format!("2026-04-{:02}", 15 - i),
            close: 100.0 * (1.01f64).powi((14 - i) as i32),
            ..Default::default()
        })
        .collect();
    // newest-first
    bars.sort_by(|a, b| b.date.cmp(&a.date));
    // Build 3 beat surprises at increasing dates (all old enough that t0_idx ≥ 10).
    let surprises = vec![
        EarningsSurprise {
            date: "2026-04-01".into(),
            symbol: "PEAD".into(),
            eps_actual: 1.10,
            eps_estimate: 1.00,
            surprise: 0.10,
            surprise_pct: 10.0,
        },
        EarningsSurprise {
            date: "2026-04-02".into(),
            symbol: "PEAD".into(),
            eps_actual: 1.20,
            eps_estimate: 1.00,
            surprise: 0.20,
            surprise_pct: 20.0,
        },
        EarningsSurprise {
            date: "2026-04-03".into(),
            symbol: "PEAD".into(),
            eps_actual: 1.15,
            eps_estimate: 1.00,
            surprise: 0.15,
            surprise_pct: 15.0,
        },
    ];
    let snap = compute_pead_snapshot("PEAD", "2026-04-15", &surprises, &bars);
    assert_eq!(snap.drift_direction_label, "DRIFT_UP");
    assert!(snap.events_used >= 3);
    assert!(snap.avg_drift_5d_pct > 2.0);
}

#[test]
fn compute_pead_no_events() {
    let snap = compute_pead_snapshot("PEAD", "2026-04-15", &[], &[]);
    assert_eq!(snap.drift_direction_label, "INSUFFICIENT_DATA");
}

// ── Round 17 tests ────────────────────────────────────────────

fn mk_mom(sym: &str, composite: f64) -> MomentumSnapshot {
    MomentumSnapshot {
        symbol: sym.into(),
        as_of: "2026-04-15".into(),
        bars_used: 252,
        composite_score: composite,
        regime_label: "STRONG".into(),
        trend_label: "STABLE".into(),
        ..Default::default()
    }
}

fn mk_pead(sym: &str, avg_5d: f64, events: usize) -> PeadSnapshot {
    PeadSnapshot {
        symbol: sym.into(),
        as_of: "2026-04-15".into(),
        num_events: events,
        events_used: events,
        avg_drift_5d_pct: avg_5d,
        drift_direction_label: if avg_5d > 0.5 {
            "DRIFT_UP".into()
        } else {
            "MIXED".into()
        },
        ..Default::default()
    }
}

fn mk_ptfs(sym: &str, f_score: i32) -> PiotroskiSnapshot {
    PiotroskiSnapshot {
        symbol: sym.into(),
        as_of: "2026-04-15".into(),
        f_score,
        strength_label: if f_score >= 7 {
            "STRONG".into()
        } else if f_score >= 4 {
            "MIXED".into()
        } else {
            "WEAK".into()
        },
        ..Default::default()
    }
}

fn mk_margins_q(sym: &str, op_margin: f64, quality: &str, trend: &str) -> MarginsSnapshot {
    MarginsSnapshot {
        symbol: sym.into(),
        as_of: "2026-04-15".into(),
        latest_operating_margin_pct: op_margin,
        quality_label: quality.into(),
        overall_trend_label: trend.into(),
        ..Default::default()
    }
}

fn mk_accruals(sym: &str, cash_conv: f64, trend: &str) -> AccrualsSnapshot {
    AccrualsSnapshot {
        symbol: sym.into(),
        as_of: "2026-04-15".into(),
        ttm_cash_conversion_pct: cash_conv,
        trend_label: trend.into(),
        ..Default::default()
    }
}

fn mk_financials_with_revenue(_sym: &str, revs: &[f64]) -> FinancialStatements {
    let income_annual: Vec<IncomeStatement> = revs
        .iter()
        .enumerate()
        .map(|(i, r)| IncomeStatement {
            date: format!("202{}-12-31", 6 - i),
            period: "FY".into(),
            revenue: *r,
            ..Default::default()
        })
        .collect();
    FinancialStatements {
        income_annual,
        ..Default::default()
    }
}

#[test]
fn sizef_snapshot_roundtrip() {
    let conn = Connection::open_in_memory().expect("open conn");
    create_research_tables_v17(&conn).expect("create v17");
    let snap = SizeFactorSnapshot {
        symbol: "SIZ".into(),
        as_of: "2026-04-15".into(),
        sector: "Technology".into(),
        market_cap: 5e11,
        tier_label: "MEGA_CAP".into(),
        percentile_rank: 92.0,
        rank_label: "TOP_DECILE".into(),
        ..Default::default()
    };
    upsert_sizef(&conn, "SIZ", &snap).unwrap();
    let got = get_sizef(&conn, "SIZ").unwrap().unwrap();
    assert_eq!(got.tier_label, "MEGA_CAP");
    assert_eq!(got.rank_label, "TOP_DECILE");
}

#[test]
fn momf_snapshot_roundtrip() {
    let conn = Connection::open_in_memory().expect("open conn");
    create_research_tables_v17(&conn).expect("create v17");
    let snap = MomentumRankSnapshot {
        symbol: "MMF".into(),
        as_of: "2026-04-15".into(),
        sector: "Energy".into(),
        composite_score: 72.0,
        rank_label: "TOP_QUARTILE".into(),
        ..Default::default()
    };
    upsert_momf(&conn, "MMF", &snap).unwrap();
    let got = get_momf(&conn, "MMF").unwrap().unwrap();
    assert_eq!(got.rank_label, "TOP_QUARTILE");
}

#[test]
fn peadrank_snapshot_roundtrip() {
    let conn = Connection::open_in_memory().expect("open conn");
    create_research_tables_v17(&conn).expect("create v17");
    let snap = PeadRankSnapshot {
        symbol: "PRK".into(),
        as_of: "2026-04-15".into(),
        sector: "Healthcare".into(),
        avg_drift_5d_pct: 2.1,
        rank_label: "ABOVE_MEDIAN".into(),
        ..Default::default()
    };
    upsert_peadrank(&conn, "PRK", &snap).unwrap();
    let got = get_peadrank(&conn, "PRK").unwrap().unwrap();
    assert_eq!(got.rank_label, "ABOVE_MEDIAN");
}

#[test]
fn fqm_snapshot_roundtrip() {
    let conn = Connection::open_in_memory().expect("open conn");
    create_research_tables_v17(&conn).expect("create v17");
    let snap = FundamentalQualityMeterSnapshot {
        symbol: "FQM".into(),
        as_of: "2026-04-15".into(),
        piotroski_score: 8,
        composite_score: 88.0,
        operator_label: "ELITE_OPERATOR".into(),
        inputs_available: 3,
        ..Default::default()
    };
    upsert_fqm(&conn, "FQM", &snap).unwrap();
    let got = get_fqm(&conn, "FQM").unwrap().unwrap();
    assert_eq!(got.operator_label, "ELITE_OPERATOR");
}

#[test]
fn revrank_snapshot_roundtrip() {
    let conn = Connection::open_in_memory().expect("open conn");
    create_research_tables_v17(&conn).expect("create v17");
    let snap = RevenueGrowthRankSnapshot {
        symbol: "RVR".into(),
        as_of: "2026-04-15".into(),
        sector: "Technology".into(),
        symbol_cagr_pct: 25.0,
        sector_median_cagr_pct: 10.0,
        gap_to_median_pp: 15.0,
        relative_label: "FAR_ABOVE".into(),
        ..Default::default()
    };
    upsert_revrank(&conn, "RVR", &snap).unwrap();
    let got = get_revrank(&conn, "RVR").unwrap().unwrap();
    assert_eq!(got.relative_label, "FAR_ABOVE");
}

#[test]
fn compute_sizef_top_decile() {
    let peers: Vec<(String, f64)> = vec![
        ("A".into(), 1e9),
        ("B".into(), 2e9),
        ("C".into(), 5e9),
        ("D".into(), 3e9),
    ];
    let snap = compute_sizef_snapshot("MEGA", "2026-04-15", "Technology", Some(5e11), &peers);
    assert_eq!(snap.tier_label, "MEGA_CAP");
    assert_eq!(snap.rank_label, "TOP_DECILE");
    assert_eq!(snap.peers_considered, 4);
}

#[test]
fn compute_sizef_no_subject() {
    let snap = compute_sizef_snapshot("NIL", "2026-04-15", "Technology", None, &[]);
    assert_eq!(snap.tier_label, "NO_DATA");
    assert_eq!(snap.rank_label, "NO_DATA");
}

#[test]
fn compute_momf_above_median() {
    let peers_owned = [
        mk_mom("A", 40.0),
        mk_mom("B", 50.0),
        mk_mom("C", 60.0),
        mk_mom("D", 70.0),
    ];
    let peers: Vec<&MomentumSnapshot> = peers_owned.iter().collect();
    let subj = mk_mom("MMF", 65.0);
    let snap = compute_momf_snapshot("MMF", "2026-04-15", "Energy", Some(&subj), &peers);
    assert!(snap.percentile_rank > 50.0);
    assert_ne!(snap.rank_label, "NO_DATA");
}

#[test]
fn compute_momf_no_subject() {
    let snap = compute_momf_snapshot("N", "2026-04-15", "S", None, &[]);
    assert_eq!(snap.rank_label, "NO_DATA");
}

#[test]
fn compute_peadrank_above_median() {
    let peers_owned = [
        mk_pead("A", 0.5, 4),
        mk_pead("B", 1.0, 5),
        mk_pead("C", 1.5, 4),
        mk_pead("D", 2.0, 4),
    ];
    let peers: Vec<&PeadSnapshot> = peers_owned.iter().collect();
    let subj = mk_pead("PRK", 1.8, 5);
    let snap = compute_peadrank_snapshot("PRK", "2026-04-15", "Healthcare", Some(&subj), &peers);
    assert!(snap.percentile_rank > 50.0);
    assert_ne!(snap.rank_label, "NO_DATA");
}

#[test]
fn compute_peadrank_insufficient() {
    let snap = compute_peadrank_snapshot("N", "2026-04-15", "S", None, &[]);
    assert_eq!(snap.rank_label, "NO_DATA");
}

#[test]
fn compute_fqm_elite_operator() {
    let p = mk_ptfs("FQM", 9);
    let m = mk_margins_q("FQM", 35.0, "HIGH", "EXPANDING");
    let a = mk_accruals("FQM", 115.0, "IMPROVING");
    let snap = compute_fqm_snapshot("FQM", "2026-04-15", Some(&p), Some(&m), Some(&a));
    assert_eq!(snap.operator_label, "ELITE_OPERATOR");
    assert_eq!(snap.inputs_available, 3);
    assert!(snap.composite_score >= 85.0);
}

#[test]
fn compute_fqm_no_inputs() {
    let snap = compute_fqm_snapshot("NIL", "2026-04-15", None, None, None);
    assert_eq!(snap.operator_label, "NO_DATA");
    assert_eq!(snap.inputs_available, 0);
}

#[test]
fn compute_revrank_far_above() {
    let subj = mk_financials_with_revenue("RVR", &[2000.0, 1600.0, 1300.0, 1000.0]);
    let peer_stmts: Vec<(String, FinancialStatements)> = vec![
        (
            "A".into(),
            mk_financials_with_revenue("A", &[1100.0, 1080.0, 1050.0, 1000.0]),
        ),
        (
            "B".into(),
            mk_financials_with_revenue("B", &[1080.0, 1060.0, 1030.0, 1000.0]),
        ),
        (
            "C".into(),
            mk_financials_with_revenue("C", &[1050.0, 1030.0, 1020.0, 1000.0]),
        ),
        (
            "D".into(),
            mk_financials_with_revenue("D", &[1070.0, 1050.0, 1030.0, 1000.0]),
        ),
    ];
    let snap =
        compute_revrank_snapshot("RVR", "2026-04-15", "Technology", Some(&subj), &peer_stmts);
    assert_eq!(snap.relative_label, "FAR_ABOVE");
    assert!(snap.symbol_cagr_pct > 20.0);
    assert!(snap.gap_to_median_pp > 15.0);
}

#[test]
fn compute_revrank_insufficient_subject() {
    let snap = compute_revrank_snapshot("NIL", "2026-04-15", "S", None, &[]);
    assert_eq!(snap.relative_label, "NO_DATA");
}

// ── Round 18 tests ────────────────────────────────────────────

fn mk_lev(sym: &str, debt: f64, equity: f64) -> LeverageSnapshot {
    LeverageSnapshot {
        symbol: sym.into(),
        as_of: "2026-04-15".into(),
        total_debt: debt,
        total_equity: equity,
        ..Default::default()
    }
}

fn mk_margins_op(sym: &str, op_margin: f64) -> MarginsSnapshot {
    MarginsSnapshot {
        symbol: sym.into(),
        as_of: "2026-04-15".into(),
        latest_operating_margin_pct: op_margin,
        overall_trend_label: "STABLE".into(),
        periods_used: 4,
        ..Default::default()
    }
}

fn mk_fqm(sym: &str, composite: f64, operator: &str) -> FundamentalQualityMeterSnapshot {
    FundamentalQualityMeterSnapshot {
        symbol: sym.into(),
        as_of: "2026-04-15".into(),
        composite_score: composite,
        operator_label: operator.into(),
        inputs_available: 3,
        ..Default::default()
    }
}

fn mk_liq(sym: &str, adv_dollar: f64, tier: &str) -> LiquiditySnapshot {
    LiquiditySnapshot {
        symbol: sym.into(),
        as_of: "2026-04-15".into(),
        avg_daily_dollar_volume: adv_dollar,
        liquidity_tier: tier.into(),
        ..Default::default()
    }
}

fn mk_surp(date: &str, surprise_pct: f64) -> EarningsSurprise {
    EarningsSurprise {
        date: date.into(),
        symbol: "X".into(),
        eps_actual: 1.0 + surprise_pct / 100.0,
        eps_estimate: 1.0,
        surprise: surprise_pct / 100.0,
        surprise_pct,
    }
}

#[test]
fn levrank_snapshot_roundtrip() {
    let conn = Connection::open_in_memory().expect("open conn");
    create_research_tables_v18(&conn).expect("create v18");
    let snap = LeverageRankSnapshot {
        symbol: "LVR".into(),
        as_of: "2026-04-15".into(),
        sector: "Industrials".into(),
        debt_to_equity: 0.3,
        rank_label: "SAFEST_QUARTILE".into(),
        ..Default::default()
    };
    upsert_levrank(&conn, "LVR", &snap).unwrap();
    let got = get_levrank(&conn, "LVR").unwrap().unwrap();
    assert_eq!(got.rank_label, "SAFEST_QUARTILE");
    assert!((got.debt_to_equity - 0.3).abs() < 1e-9);
}

#[test]
fn operank_snapshot_roundtrip() {
    let conn = Connection::open_in_memory().expect("open conn");
    create_research_tables_v18(&conn).expect("create v18");
    let snap = OperatingQualityRankSnapshot {
        symbol: "OPR".into(),
        as_of: "2026-04-15".into(),
        sector: "Technology".into(),
        operating_margin_pct: 35.0,
        rank_label: "TOP_DECILE".into(),
        ..Default::default()
    };
    upsert_operank(&conn, "OPR", &snap).unwrap();
    let got = get_operank(&conn, "OPR").unwrap().unwrap();
    assert_eq!(got.rank_label, "TOP_DECILE");
}

#[test]
fn fqmrank_snapshot_roundtrip() {
    let conn = Connection::open_in_memory().expect("open conn");
    create_research_tables_v18(&conn).expect("create v18");
    let snap = FqmRankSnapshot {
        symbol: "FQR".into(),
        as_of: "2026-04-15".into(),
        sector: "Healthcare".into(),
        composite_score: 85.0,
        operator_label: "ELITE_OPERATOR".into(),
        rank_label: "TOP_DECILE".into(),
        ..Default::default()
    };
    upsert_fqmrank(&conn, "FQR", &snap).unwrap();
    let got = get_fqmrank(&conn, "FQR").unwrap().unwrap();
    assert_eq!(got.operator_label, "ELITE_OPERATOR");
    assert_eq!(got.rank_label, "TOP_DECILE");
}

#[test]
fn liqrank_snapshot_roundtrip() {
    let conn = Connection::open_in_memory().expect("open conn");
    create_research_tables_v18(&conn).expect("create v18");
    let snap = LiquidityRankSnapshot {
        symbol: "LQR".into(),
        as_of: "2026-04-15".into(),
        sector: "Financials".into(),
        avg_daily_dollar_volume: 2.5e9,
        tier_label: "DEEP".into(),
        rank_label: "TOP_QUARTILE".into(),
        ..Default::default()
    };
    upsert_liqrank(&conn, "LQR", &snap).unwrap();
    let got = get_liqrank(&conn, "LQR").unwrap().unwrap();
    assert_eq!(got.tier_label, "DEEP");
    assert_eq!(got.rank_label, "TOP_QUARTILE");
}

#[test]
fn surpstk_snapshot_roundtrip() {
    let conn = Connection::open_in_memory().expect("open conn");
    create_research_tables_v18(&conn).expect("create v18");
    let snap = EarningsSurpriseStreakSnapshot {
        symbol: "SUR".into(),
        as_of: "2026-04-15".into(),
        total_events: 8,
        beats: 7,
        misses: 1,
        beat_rate_pct: 87.5,
        current_streak_type: "BEAT".into(),
        current_streak_len: 5,
        streak_label: "HOT_STREAK".into(),
        ..Default::default()
    };
    upsert_surpstk(&conn, "SUR", &snap).unwrap();
    let got = get_surpstk(&conn, "SUR").unwrap().unwrap();
    assert_eq!(got.streak_label, "HOT_STREAK");
    assert_eq!(got.beats, 7);
}

#[test]
fn compute_levrank_safest_decile() {
    // Subject has the LOWEST D/E in sector → should rank safest.
    let peers_owned = [
        mk_lev("A", 500.0, 1000.0),  // D/E 0.50
        mk_lev("B", 800.0, 1000.0),  // D/E 0.80
        mk_lev("C", 1200.0, 1000.0), // D/E 1.20
        mk_lev("D", 1500.0, 1000.0), // D/E 1.50
    ];
    let peers: Vec<&LeverageSnapshot> = peers_owned.iter().collect();
    let subj = mk_lev("LVR", 100.0, 1000.0); // D/E 0.10
    let snap = compute_levrank_snapshot("LVR", "2026-04-15", "Industrials", Some(&subj), &peers);
    assert_eq!(snap.rank_label, "SAFEST_DECILE");
    assert!(snap.percentile_rank >= 90.0);
    assert_eq!(snap.rank_position, 1);
}

#[test]
fn compute_levrank_negative_equity() {
    let subj = mk_lev("NEG", 500.0, -100.0);
    let snap = compute_levrank_snapshot("NEG", "2026-04-15", "S", Some(&subj), &[]);
    assert_eq!(snap.rank_label, "NEGATIVE_EQUITY");
}

#[test]
fn compute_levrank_no_subject() {
    let snap = compute_levrank_snapshot("NIL", "2026-04-15", "S", None, &[]);
    assert_eq!(snap.rank_label, "NO_DATA");
}

#[test]
fn compute_operank_top_decile() {
    let peers_owned = [
        mk_margins_op("A", 5.0),
        mk_margins_op("B", 10.0),
        mk_margins_op("C", 15.0),
        mk_margins_op("D", 20.0),
    ];
    let peers: Vec<&MarginsSnapshot> = peers_owned.iter().collect();
    let subj = mk_margins_op("OPR", 45.0);
    let snap = compute_operank_snapshot("OPR", "2026-04-15", "Technology", Some(&subj), &peers);
    assert_eq!(snap.rank_label, "TOP_DECILE");
    assert!(snap.percentile_rank >= 90.0);
}

#[test]
fn compute_operank_no_subject() {
    let snap = compute_operank_snapshot("NIL", "2026-04-15", "S", None, &[]);
    assert_eq!(snap.rank_label, "NO_DATA");
}

#[test]
fn compute_fqmrank_top_decile() {
    let peers_owned = [
        mk_fqm("A", 40.0, "WEAK_OPERATOR"),
        mk_fqm("B", 55.0, "AVERAGE_OPERATOR"),
        mk_fqm("C", 65.0, "STRONG_OPERATOR"),
        mk_fqm("D", 72.0, "STRONG_OPERATOR"),
    ];
    let peers: Vec<&FundamentalQualityMeterSnapshot> = peers_owned.iter().collect();
    let subj = mk_fqm("FQR", 92.0, "ELITE_OPERATOR");
    let snap = compute_fqmrank_snapshot("FQR", "2026-04-15", "Technology", Some(&subj), &peers);
    assert_eq!(snap.rank_label, "TOP_DECILE");
    assert_eq!(snap.operator_label, "ELITE_OPERATOR");
}

#[test]
fn compute_fqmrank_filters_no_data_peers() {
    let peers_owned = [
        mk_fqm("A", 0.0, "NO_DATA"),
        mk_fqm("B", 0.0, "NO_DATA"),
        mk_fqm("C", 0.0, "NO_DATA"),
        mk_fqm("D", 0.0, "NO_DATA"),
    ];
    let peers: Vec<&FundamentalQualityMeterSnapshot> = peers_owned.iter().collect();
    let subj = mk_fqm("FQR", 90.0, "ELITE_OPERATOR");
    let snap = compute_fqmrank_snapshot("FQR", "2026-04-15", "T", Some(&subj), &peers);
    assert_eq!(snap.rank_label, "NO_DATA");
}

#[test]
fn compute_liqrank_deepest() {
    let peers_owned = [
        mk_liq("A", 1e6, "THIN"),
        mk_liq("B", 5e7, "MODERATE"),
        mk_liq("C", 2e8, "LIQUID"),
        mk_liq("D", 8e8, "LIQUID"),
    ];
    let peers: Vec<&LiquiditySnapshot> = peers_owned.iter().collect();
    let subj = mk_liq("LQR", 5e9, "DEEP");
    let snap = compute_liqrank_snapshot("LQR", "2026-04-15", "Financials", Some(&subj), &peers);
    assert_eq!(snap.rank_label, "TOP_DECILE");
    assert_eq!(snap.rank_position, 1);
    assert_eq!(snap.tier_label, "DEEP");
}

#[test]
fn compute_liqrank_filters_insufficient_data() {
    let peers_owned = [
        mk_liq("A", 0.0, "INSUFFICIENT_DATA"),
        mk_liq("B", 0.0, "INSUFFICIENT_DATA"),
        mk_liq("C", 0.0, "INSUFFICIENT_DATA"),
    ];
    let peers: Vec<&LiquiditySnapshot> = peers_owned.iter().collect();
    let subj = mk_liq("LQR", 1e9, "DEEP");
    let snap = compute_liqrank_snapshot("LQR", "2026-04-15", "Financials", Some(&subj), &peers);
    assert_eq!(snap.rank_label, "NO_DATA");
}

#[test]
fn compute_surpstk_hot_streak() {
    let rows = vec![
        mk_surp("2026-04-01", 12.0),
        mk_surp("2026-01-01", 9.0),
        mk_surp("2025-10-01", 6.0),
        mk_surp("2025-07-01", 4.0),
        mk_surp("2025-04-01", 5.0),
        mk_surp("2025-01-01", 3.0),
        mk_surp("2024-10-01", 1.0),
        mk_surp("2024-07-01", -3.0),
    ];
    let snap = compute_surpstk_snapshot("HOT", "2026-04-15", &rows);
    assert_eq!(snap.streak_label, "HOT_STREAK");
    assert_eq!(snap.current_streak_type, "BEAT");
    assert!(snap.current_streak_len >= 3);
    assert!(snap.beat_rate_pct >= 75.0);
}

#[test]
fn compute_surpstk_cold_streak() {
    let rows = vec![
        mk_surp("2026-04-01", -8.0),
        mk_surp("2026-01-01", -5.0),
        mk_surp("2025-10-01", -4.0),
        mk_surp("2025-07-01", -3.0),
        mk_surp("2025-04-01", -6.0),
        mk_surp("2025-01-01", -2.5),
        mk_surp("2024-10-01", 1.0),
        mk_surp("2024-07-01", 2.5),
    ];
    let snap = compute_surpstk_snapshot("CLD", "2026-04-15", &rows);
    assert_eq!(snap.streak_label, "COLD_STREAK");
    assert_eq!(snap.current_streak_type, "MISS");
    assert!(snap.current_streak_len >= 3);
}

#[test]
fn compute_surpstk_mixed() {
    // 50% beat rate, alternating → neither BEAT_TREND nor MISS_TREND.
    let rows = vec![
        mk_surp("2026-04-01", 3.0),
        mk_surp("2026-01-01", -3.0),
        mk_surp("2025-10-01", 3.0),
        mk_surp("2025-07-01", -3.0),
        mk_surp("2025-04-01", 3.0),
        mk_surp("2025-01-01", -3.0),
    ];
    let snap = compute_surpstk_snapshot("MIX", "2026-04-15", &rows);
    assert_eq!(snap.streak_label, "MIXED");
    assert_eq!(snap.beats, 3);
    assert_eq!(snap.misses, 3);
}

#[test]
fn compute_surpstk_insufficient_data() {
    let snap = compute_surpstk_snapshot("NIL", "2026-04-15", &[]);
    assert_eq!(snap.streak_label, "INSUFFICIENT_DATA");
}

// ── Round 19 tests ────────────────────────────────────────────

fn mk_divg(sym: &str, cagr3: f64, trend: &str) -> DivgSnapshot {
    DivgSnapshot {
        symbol: sym.into(),
        as_of: "2026-04-15".into(),
        total_payments: 12,
        years_covered: 10,
        cagr_3y_pct: cagr3,
        consecutive_growth_years: 5,
        trend_label: trend.into(),
        ..Default::default()
    }
}

fn mk_earm(sym: &str, score: f64, label: &str) -> EarmSnapshot {
    EarmSnapshot {
        symbol: sym.into(),
        as_of: "2026-04-15".into(),
        quarters_used: 8,
        composite_score: score,
        momentum_label: label.into(),
        ..Default::default()
    }
}

fn mk_updm(sym: &str, net90: i32, bias: &str) -> UpdmSnapshot {
    UpdmSnapshot {
        symbol: sym.into(),
        as_of: "2026-04-15".into(),
        total_actions: 10,
        net_90d: net90,
        bias_label: bias.into(),
        ..Default::default()
    }
}

fn mk_hp(date: &str, open: f64, high: f64, low: f64, close: f64) -> HistoricalPriceRow {
    HistoricalPriceRow {
        date: date.into(),
        open,
        high,
        low,
        close,
        adj_close: close,
        volume: 1_000_000.0,
        change: close - open,
        change_pct: 0.0,
    }
}

#[test]
fn dvdrank_snapshot_roundtrip() {
    let c = rusqlite::Connection::open_in_memory().unwrap();
    create_research_tables_v19(&c).unwrap();
    let snap = DividendGrowthRankSnapshot {
        symbol: "AAA".into(),
        as_of: "2026-04-15".into(),
        sector: "Tech".into(),
        cagr_3y_pct: 12.5,
        rank_label: "TOP_DECILE".into(),
        ..Default::default()
    };
    upsert_dvdrank(&c, "AAA", &snap).unwrap();
    let got = get_dvdrank(&c, "AAA").unwrap().unwrap();
    assert_eq!(got.rank_label, "TOP_DECILE");
}

#[test]
fn earmrank_snapshot_roundtrip() {
    let c = rusqlite::Connection::open_in_memory().unwrap();
    create_research_tables_v19(&c).unwrap();
    let snap = EarningsMomentumRankSnapshot {
        symbol: "AAA".into(),
        as_of: "2026-04-15".into(),
        sector: "Tech".into(),
        composite_score: 72.0,
        rank_label: "ABOVE_MEDIAN".into(),
        ..Default::default()
    };
    upsert_earmrank(&c, "AAA", &snap).unwrap();
    let got = get_earmrank(&c, "AAA").unwrap().unwrap();
    assert_eq!(got.rank_label, "ABOVE_MEDIAN");
}

#[test]
fn updgrank_snapshot_roundtrip() {
    let c = rusqlite::Connection::open_in_memory().unwrap();
    create_research_tables_v19(&c).unwrap();
    let snap = UpgradeDowngradeRankSnapshot {
        symbol: "AAA".into(),
        as_of: "2026-04-15".into(),
        sector: "Tech".into(),
        net_90d: 5,
        rank_label: "BELOW_MEDIAN".into(),
        ..Default::default()
    };
    upsert_updgrank(&c, "AAA", &snap).unwrap();
    let got = get_updgrank(&c, "AAA").unwrap().unwrap();
    assert_eq!(got.net_90d, 5);
}

#[test]
fn gy_snapshot_roundtrip() {
    let c = rusqlite::Connection::open_in_memory().unwrap();
    create_research_tables_v19(&c).unwrap();
    let snap = GapYearlySnapshot {
        symbol: "AAA".into(),
        as_of: "2026-04-15".into(),
        gaps_total: 8,
        gap_label: "NORMAL".into(),
        ..Default::default()
    };
    upsert_gy(&c, "AAA", &snap).unwrap();
    let got = get_gy(&c, "AAA").unwrap().unwrap();
    assert_eq!(got.gap_label, "NORMAL");
}

#[test]
fn des_snapshot_roundtrip() {
    let c = rusqlite::Connection::open_in_memory().unwrap();
    create_research_tables_v19(&c).unwrap();
    let snap = DailyEventStreakSnapshot {
        symbol: "AAA".into(),
        as_of: "2026-04-15".into(),
        bars_used: 252,
        current_streak_type: "UP".into(),
        current_streak_len: 3,
        streak_label: "STRONG_UPTREND".into(),
        ..Default::default()
    };
    upsert_des(&c, "AAA", &snap).unwrap();
    let got = get_des(&c, "AAA").unwrap().unwrap();
    assert_eq!(got.streak_label, "STRONG_UPTREND");
}

#[test]
fn compute_dvdrank_top_decile() {
    let subj = mk_divg("AAA", 15.0, "GROWING");
    let p1 = mk_divg("BBB", 3.0, "GROWING");
    let p2 = mk_divg("CCC", 5.0, "GROWING");
    let p3 = mk_divg("DDD", 2.0, "STABLE");
    let p4 = mk_divg("EEE", 1.0, "GROWING");
    let peers = vec![&p1, &p2, &p3, &p4];
    let snap = compute_dvdrank_snapshot("AAA", "2026-04-15", "Tech", Some(&subj), &peers);
    assert_eq!(snap.rank_label, "TOP_DECILE");
    assert!(snap.percentile_rank >= 90.0);
}

#[test]
fn compute_dvdrank_no_history_filtered() {
    let subj = mk_divg("AAA", 5.0, "GROWING");
    let bad = mk_divg("BBB", 0.0, "NO_HISTORY");
    let ok1 = mk_divg("CCC", 3.0, "STABLE");
    let ok2 = mk_divg("DDD", 2.0, "GROWING");
    let ok3 = mk_divg("EEE", 4.0, "STABLE");
    let peers = vec![&bad, &ok1, &ok2, &ok3];
    let snap = compute_dvdrank_snapshot("AAA", "2026-04-15", "Tech", Some(&subj), &peers);
    assert_eq!(snap.peers_considered, 4);
    assert_eq!(snap.peers_with_data, 3);
    assert!(snap.rank_label != "INSUFFICIENT_DATA");
    assert!(snap.rank_label != "NO_DATA");
}

#[test]
fn compute_earmrank_above_median() {
    let subj = mk_earm("AAA", 75.0, "ACCELERATING");
    let p1 = mk_earm("BBB", 40.0, "STABLE");
    let p2 = mk_earm("CCC", 50.0, "STABLE");
    let p3 = mk_earm("DDD", 60.0, "ACCELERATING");
    let p4 = mk_earm("EEE", 30.0, "DECELERATING");
    let peers = vec![&p1, &p2, &p3, &p4];
    let snap = compute_earmrank_snapshot("AAA", "2026-04-15", "Tech", Some(&subj), &peers);
    assert!(snap.percentile_rank > 50.0);
    assert_eq!(snap.composite_score, 75.0);
}

#[test]
fn compute_earmrank_insufficient_filtered() {
    let subj = mk_earm("AAA", 65.0, "STABLE");
    let bad = mk_earm("BBB", 0.0, "INSUFFICIENT_DATA");
    let peers = vec![&bad];
    let snap = compute_earmrank_snapshot("AAA", "2026-04-15", "Tech", Some(&subj), &peers);
    assert_eq!(snap.rank_label, "INSUFFICIENT_DATA");
}

#[test]
fn compute_updgrank_bullish() {
    let subj = mk_updm("AAA", 8, "BULLISH");
    let p1 = mk_updm("BBB", -2, "BEARISH");
    let p2 = mk_updm("CCC", 1, "NEUTRAL");
    let p3 = mk_updm("DDD", 3, "BULLISH");
    let p4 = mk_updm("EEE", -5, "BEARISH");
    let peers = vec![&p1, &p2, &p3, &p4];
    let snap = compute_updgrank_snapshot("AAA", "2026-04-15", "Tech", Some(&subj), &peers);
    assert_eq!(snap.net_90d, 8);
    assert!(snap.percentile_rank > 60.0);
}

#[test]
fn compute_updgrank_no_coverage_filtered() {
    let subj = mk_updm("AAA", 3, "BULLISH");
    let bad = mk_updm("BBB", 0, "NO_COVERAGE");
    let ok = mk_updm("CCC", 0, "NEUTRAL");
    let peers = vec![&bad, &ok];
    let snap = compute_updgrank_snapshot("AAA", "2026-04-15", "Tech", Some(&subj), &peers);
    assert_eq!(snap.peers_with_data, 1);
}

#[test]
fn compute_gy_normal() {
    // 30-bar window, one small up gap, one small down gap, rest flat.
    let mut bars = Vec::new();
    for i in 0..30 {
        let date = format!("2025-01-{:02}", i + 1);
        // Day 5: prev close 100, today open 102.5 → +2.5% gap
        // Day 10: prev close 100, today open 97.5 → -2.5% gap
        let open = if i == 5 {
            102.5
        } else if i == 10 {
            97.5
        } else {
            100.0
        };
        bars.push(mk_hp(&date, open, open + 1.0, open - 1.0, 100.0));
    }
    let snap = compute_gy_snapshot("AAA", "2026-04-15", &bars);
    assert!(snap.bars_used >= 20);
    assert!(snap.gaps_up_2pct >= 1);
    assert!(snap.gaps_down_2pct >= 1);
}

#[test]
fn compute_gy_explosive() {
    // 30-bar window with a single 12% gap up → EXPLOSIVE via 10% band.
    let mut bars = Vec::new();
    for i in 0..30 {
        let date = format!("2025-02-{:02}", i + 1);
        let open = if i == 15 { 112.0 } else { 100.0 };
        bars.push(mk_hp(&date, open, open + 1.0, open - 1.0, 100.0));
    }
    let snap = compute_gy_snapshot("AAA", "2026-04-15", &bars);
    assert_eq!(snap.gap_label, "EXPLOSIVE");
    assert_eq!(snap.gaps_up_10pct, 1);
}

#[test]
fn compute_gy_insufficient() {
    let bars = vec![mk_hp("2025-03-01", 100.0, 101.0, 99.0, 100.0)];
    let snap = compute_gy_snapshot("AAA", "2026-04-15", &bars);
    assert_eq!(snap.gap_label, "INSUFFICIENT_DATA");
}

#[test]
fn compute_des_uptrend() {
    // 30 bars, strictly rising close each day → STRONG_UPTREND.
    let mut bars = Vec::new();
    for i in 0..30 {
        let date = format!("2025-04-{:02}", i + 1);
        let close = 100.0 + i as f64;
        bars.push(mk_hp(&date, close - 0.5, close + 0.5, close - 0.5, close));
    }
    let snap = compute_des_snapshot("AAA", "2026-04-15", &bars);
    assert_eq!(snap.streak_label, "STRONG_UPTREND");
    assert_eq!(snap.current_streak_type, "UP");
    assert!(snap.longest_up_streak >= 5);
}

#[test]
fn compute_des_downtrend() {
    let mut bars = Vec::new();
    for i in 0..30 {
        let date = format!("2025-05-{:02}", i + 1);
        let close = 200.0 - i as f64;
        bars.push(mk_hp(&date, close + 0.5, close + 0.5, close - 0.5, close));
    }
    let snap = compute_des_snapshot("AAA", "2026-04-15", &bars);
    assert_eq!(snap.streak_label, "STRONG_DOWNTREND");
    assert_eq!(snap.current_streak_type, "DOWN");
}

#[test]
fn compute_des_insufficient() {
    let bars = vec![mk_hp("2025-06-01", 100.0, 101.0, 99.0, 100.0)];
    let snap = compute_des_snapshot("AAA", "2026-04-15", &bars);
    assert_eq!(snap.streak_label, "INSUFFICIENT_DATA");
}

// ── Round 20 tests ────────────────────────────────────────────

#[test]
fn dvdyieldrank_snapshot_roundtrip() {
    let c = rusqlite::Connection::open_in_memory().unwrap();
    create_research_tables_v20(&c).unwrap();
    let snap = DividendYieldRankSnapshot {
        symbol: "AAA".into(),
        as_of: "2026-04-15".into(),
        sector: "Utilities".into(),
        dividend_yield_pct: 4.5,
        rank_label: "TOP_DECILE".into(),
        ..Default::default()
    };
    upsert_dvdyieldrank(&c, "AAA", &snap).unwrap();
    let got = get_dvdyieldrank(&c, "AAA").unwrap().unwrap();
    assert_eq!(got.rank_label, "TOP_DECILE");
    assert_eq!(got.dividend_yield_pct, 4.5);
}

#[test]
fn shrank_snapshot_roundtrip() {
    let c = rusqlite::Connection::open_in_memory().unwrap();
    create_research_tables_v20(&c).unwrap();
    let snap = ShortInterestRankSnapshot {
        symbol: "AAA".into(),
        as_of: "2026-04-15".into(),
        sector: "Tech".into(),
        short_pct_of_float: 2.5,
        rank_label: "SAFEST_DECILE".into(),
        ..Default::default()
    };
    upsert_shrank(&c, "AAA", &snap).unwrap();
    let got = get_shrank(&c, "AAA").unwrap().unwrap();
    assert_eq!(got.rank_label, "SAFEST_DECILE");
    assert_eq!(got.short_pct_of_float, 2.5);
}

#[test]
fn atrann_snapshot_roundtrip() {
    let c = rusqlite::Connection::open_in_memory().unwrap();
    create_research_tables_v20(&c).unwrap();
    let snap = AnnualizedAtrSnapshot {
        symbol: "AAA".into(),
        as_of: "2026-04-15".into(),
        bars_used: 253,
        latest_close: 100.0,
        atr14: 1.5,
        atr14_pct: 1.5,
        atr_annualized_pct: 23.8,
        regime_label: "NORMAL_VOL".into(),
        ..Default::default()
    };
    upsert_atrann(&c, "AAA", &snap).unwrap();
    let got = get_atrann(&c, "AAA").unwrap().unwrap();
    assert_eq!(got.regime_label, "NORMAL_VOL");
    assert!((got.atr_annualized_pct - 23.8).abs() < 1e-9);
}

#[test]
fn ddhist_snapshot_roundtrip() {
    let c = rusqlite::Connection::open_in_memory().unwrap();
    create_research_tables_v20(&c).unwrap();
    let snap = DrawdownHistorySnapshot {
        symbol: "AAA".into(),
        as_of: "2026-04-15".into(),
        bars_used: 253,
        max_drawdown_pct: -12.0,
        longest_drawdown_days: 45,
        corrections_5pct: 3,
        corrections_10pct: 1,
        current_drawdown_pct: -2.0,
        regime_label: "MEANINGFUL".into(),
        ..Default::default()
    };
    upsert_ddhist(&c, "AAA", &snap).unwrap();
    let got = get_ddhist(&c, "AAA").unwrap().unwrap();
    assert_eq!(got.regime_label, "MEANINGFUL");
    assert_eq!(got.corrections_5pct, 3);
}

#[test]
fn priceperf_snapshot_roundtrip() {
    let c = rusqlite::Connection::open_in_memory().unwrap();
    create_research_tables_v20(&c).unwrap();
    let snap = PricePerformanceSnapshot {
        symbol: "AAA".into(),
        as_of: "2026-04-15".into(),
        bars_used: 253,
        latest_close: 120.0,
        ret_1m_pct: 2.5,
        ret_3m_pct: 8.0,
        ret_6m_pct: 12.0,
        ret_ytd_pct: 15.0,
        ret_1y_pct: 20.0,
        trend_label: "BULL".into(),
        ..Default::default()
    };
    upsert_priceperf(&c, "AAA", &snap).unwrap();
    let got = get_priceperf(&c, "AAA").unwrap().unwrap();
    assert_eq!(got.trend_label, "BULL");
    assert!((got.ret_1y_pct - 20.0).abs() < 1e-9);
}

#[test]
fn compute_dvdyieldrank_top_decile() {
    let peers = vec![
        ("BBB".to_string(), Some(1.5)),
        ("CCC".to_string(), Some(2.0)),
        ("DDD".to_string(), Some(2.5)),
        ("EEE".to_string(), Some(1.0)),
    ];
    let snap = compute_dvdyieldrank_snapshot("AAA", "2026-04-15", "Utilities", Some(6.0), &peers);
    assert!(snap.percentile_rank >= 90.0);
    assert_eq!(snap.rank_label, "TOP_DECILE");
    assert_eq!(snap.peers_with_data, 4);
}

#[test]
fn compute_dvdyieldrank_non_payer_filtered() {
    let peers = vec![
        ("BBB".to_string(), None),      // non-payer
        ("CCC".to_string(), Some(0.0)), // non-payer (zero yield)
        ("DDD".to_string(), Some(3.0)),
        ("EEE".to_string(), Some(4.0)),
        ("FFF".to_string(), Some(2.0)),
    ];
    let snap = compute_dvdyieldrank_snapshot("AAA", "2026-04-15", "Utilities", Some(5.0), &peers);
    assert_eq!(snap.peers_considered, 5);
    assert_eq!(snap.peers_with_data, 3);
    assert_ne!(snap.rank_label, "INSUFFICIENT_DATA");
    assert_ne!(snap.rank_label, "NO_DATA");
}

#[test]
fn compute_dvdyieldrank_subject_non_payer() {
    let peers = vec![
        ("BBB".to_string(), Some(3.0)),
        ("CCC".to_string(), Some(4.0)),
        ("DDD".to_string(), Some(2.0)),
    ];
    let snap = compute_dvdyieldrank_snapshot("AAA", "2026-04-15", "Tech", Some(0.0), &peers);
    assert_eq!(snap.rank_label, "NO_DATA");
}

#[test]
fn compute_shrank_safest_decile() {
    // Subject has lowest short interest → risk-inverted top rank (SAFEST).
    let peers = vec![
        ("BBB".to_string(), Some(8.0)),
        ("CCC".to_string(), Some(10.0)),
        ("DDD".to_string(), Some(12.0)),
        ("EEE".to_string(), Some(15.0)),
    ];
    let snap = compute_shrank_snapshot("AAA", "2026-04-15", "Tech", Some(1.0), &peers);
    assert!(snap.percentile_rank >= 90.0);
    assert_eq!(snap.rank_label, "SAFEST_DECILE");
    assert_eq!(snap.short_pct_of_float, 1.0);
}

#[test]
fn compute_shrank_riskiest_decile() {
    // Subject has highest short interest → risk-inverted bottom rank (RISKIEST).
    // Need ≥10 peers so the floor 0.5/total*100 is strictly below 10.
    let peers = vec![
        ("BBB".to_string(), Some(2.0)),
        ("CCC".to_string(), Some(3.0)),
        ("DDD".to_string(), Some(4.0)),
        ("EEE".to_string(), Some(5.0)),
        ("FFF".to_string(), Some(6.0)),
        ("GGG".to_string(), Some(7.0)),
        ("HHH".to_string(), Some(8.0)),
        ("III".to_string(), Some(9.0)),
        ("JJJ".to_string(), Some(10.0)),
        ("KKK".to_string(), Some(11.0)),
    ];
    let snap = compute_shrank_snapshot("AAA", "2026-04-15", "Tech", Some(25.0), &peers);
    assert!(snap.percentile_rank < 10.0);
    assert_eq!(snap.rank_label, "RISKIEST_DECILE");
}

#[test]
fn compute_shrank_insufficient() {
    let peers = vec![("BBB".to_string(), None), ("CCC".to_string(), Some(5.0))];
    let snap = compute_shrank_snapshot("AAA", "2026-04-15", "Tech", Some(3.0), &peers);
    assert_eq!(snap.rank_label, "INSUFFICIENT_DATA");
}

#[test]
fn compute_atrann_low_vol() {
    // 30 quiet bars (≤0.5% HL range) → LOW_VOL regime.
    let mut bars = Vec::new();
    for i in 0..30 {
        let date = format!("2025-01-{:02}", i + 1);
        bars.push(mk_hp(&date, 100.0, 100.3, 99.7, 100.0));
    }
    let snap = compute_atrann_snapshot("AAA", "2026-04-15", &bars);
    assert_eq!(snap.regime_label, "LOW_VOL");
    assert!(snap.atr_annualized_pct < 15.0);
}

#[test]
fn compute_atrann_high_vol() {
    // 30 wild bars (5% HL range) → HIGH_VOL or EXTREME_VOL.
    let mut bars = Vec::new();
    for i in 0..30 {
        let date = format!("2025-02-{:02}", i + 1);
        bars.push(mk_hp(&date, 100.0, 103.0, 97.0, 100.0));
    }
    let snap = compute_atrann_snapshot("AAA", "2026-04-15", &bars);
    assert!(
        snap.atr_annualized_pct > 30.0,
        "expected > 30% annualized, got {}",
        snap.atr_annualized_pct
    );
    assert!(snap.regime_label == "HIGH_VOL" || snap.regime_label == "EXTREME_VOL");
}

#[test]
fn compute_atrann_insufficient() {
    let bars = vec![mk_hp("2025-03-01", 100.0, 101.0, 99.0, 100.0)];
    let snap = compute_atrann_snapshot("AAA", "2026-04-15", &bars);
    assert_eq!(snap.regime_label, "INSUFFICIENT_DATA");
}

#[test]
fn compute_ddhist_shallow() {
    // 30 quiet bars, max 3% dip → SHALLOW regime.
    let mut bars = Vec::new();
    for i in 0..30 {
        let date = format!("2025-04-{:02}", i + 1);
        let c = if (5..15).contains(&i) { 98.0 } else { 100.0 };
        bars.push(mk_hp(&date, c, c + 0.5, c - 0.5, c));
    }
    let snap = compute_ddhist_snapshot("AAA", "2026-04-15", &bars);
    assert!(snap.max_drawdown_pct > -5.0);
    assert!(snap.regime_label == "SHALLOW" || snap.regime_label == "RECOVERING");
}

#[test]
fn compute_ddhist_severe() {
    // Rise to peak, then 25% decline and partial recovery → SEVERE.
    let mut bars = Vec::new();
    for i in 0..20 {
        let date = format!("2025-05-{:02}", i + 1);
        bars.push(mk_hp(&date, 100.0, 101.0, 99.0, 100.0));
    }
    // Peak at 120
    for i in 0..10 {
        let date = format!("2025-06-{:02}", i + 1);
        let c = 100.0 + (i as f64 + 1.0) * 2.0;
        bars.push(mk_hp(&date, c, c + 0.5, c - 0.5, c));
    }
    // Crash down 25% to 90
    for i in 0..10 {
        let date = format!("2025-07-{:02}", i + 1);
        let c = 120.0 - (i as f64 + 1.0) * 3.0;
        bars.push(mk_hp(&date, c, c + 0.5, c - 0.5, c));
    }
    let snap = compute_ddhist_snapshot("AAA", "2026-04-15", &bars);
    assert!(
        snap.max_drawdown_pct < -20.0,
        "expected < -20%, got {}",
        snap.max_drawdown_pct
    );
    assert!(
        snap.regime_label == "SEVERE"
            || snap.regime_label == "CATASTROPHIC"
            || snap.regime_label == "MEANINGFUL"
    );
}

#[test]
fn compute_priceperf_bull() {
    // Sustained 25%+ rally over the window → BULL or STRONG_BULL.
    let mut bars = Vec::new();
    for i in 0..260 {
        let date = format!("2025-{:02}-{:02}", (i / 20) + 1, (i % 20) + 1);
        let c = 100.0 + i as f64 * 0.15; // ~39% rise over window
        bars.push(mk_hp(&date, c, c + 0.2, c - 0.2, c));
    }
    let snap = compute_priceperf_snapshot("AAA", "2026-04-15", &bars);
    assert!(snap.ret_1y_pct > 10.0);
    assert!(snap.trend_label == "BULL" || snap.trend_label == "STRONG_BULL");
}

#[test]
fn compute_priceperf_bear() {
    // Sustained decline → BEAR or STRONG_BEAR.
    let mut bars = Vec::new();
    for i in 0..260 {
        let date = format!("2025-{:02}-{:02}", (i / 20) + 1, (i % 20) + 1);
        let c = 200.0 - i as f64 * 0.3; // ~39% decline
        bars.push(mk_hp(&date, c, c + 0.2, c - 0.2, c));
    }
    let snap = compute_priceperf_snapshot("AAA", "2026-04-15", &bars);
    assert!(snap.ret_1y_pct < -10.0);
    assert!(snap.trend_label == "BEAR" || snap.trend_label == "STRONG_BEAR");
}

#[test]
fn compute_priceperf_insufficient() {
    let bars = vec![mk_hp("2025-06-01", 100.0, 101.0, 99.0, 100.0)];
    let snap = compute_priceperf_snapshot("AAA", "2026-04-15", &bars);
    assert_eq!(snap.trend_label, "INSUFFICIENT_DATA");
}

// ── Round 21 tests ──

#[test]
fn betarank_snapshot_roundtrip() {
    let c = rusqlite::Connection::open_in_memory().unwrap();
    create_research_tables_v21(&c).unwrap();
    let snap = BetaRankSnapshot {
        symbol: "AAA".into(),
        as_of: "2026-04-15".into(),
        sector: "Technology".into(),
        subject_beta: Some(0.8),
        percentile_rank: 85.0,
        rank_label: "SAFEST_QUARTILE".into(),
        ..Default::default()
    };
    upsert_betarank(&c, "AAA", &snap).unwrap();
    let got = get_betarank(&c, "AAA").unwrap().unwrap();
    assert_eq!(got.rank_label, "SAFEST_QUARTILE");
    assert!((got.subject_beta.unwrap() - 0.8).abs() < 1e-9);
}

#[test]
fn pegrank_snapshot_roundtrip() {
    let c = rusqlite::Connection::open_in_memory().unwrap();
    create_research_tables_v21(&c).unwrap();
    let snap = PegRankSnapshot {
        symbol: "BBB".into(),
        as_of: "2026-04-15".into(),
        sector: "Technology".into(),
        subject_peg: Some(0.9),
        percentile_rank: 90.0,
        rank_label: "TOP_DECILE".into(),
        ..Default::default()
    };
    upsert_pegrank(&c, "BBB", &snap).unwrap();
    let got = get_pegrank(&c, "BBB").unwrap().unwrap();
    assert_eq!(got.rank_label, "TOP_DECILE");
}

#[test]
fn fhighlow_snapshot_roundtrip() {
    let c = rusqlite::Connection::open_in_memory().unwrap();
    create_research_tables_v21(&c).unwrap();
    let snap = FiftyTwoWeekHighLowSnapshot {
        symbol: "CCC".into(),
        as_of: "2026-04-15".into(),
        bars_used: 253,
        latest_close: 120.0,
        high_52w: 150.0,
        high_52w_date: "2025-11-01".into(),
        days_since_high: 100,
        low_52w: 80.0,
        low_52w_date: "2025-07-01".into(),
        days_since_low: 200,
        pct_from_high: -20.0,
        pct_from_low: 50.0,
        range_position_pct: 57.0,
        proximity_label: "MID_RANGE".into(),
        ..Default::default()
    };
    upsert_fhighlow(&c, "CCC", &snap).unwrap();
    let got = get_fhighlow(&c, "CCC").unwrap().unwrap();
    assert_eq!(got.proximity_label, "MID_RANGE");
}

#[test]
fn rvcone_snapshot_roundtrip() {
    let c = rusqlite::Connection::open_in_memory().unwrap();
    create_research_tables_v21(&c).unwrap();
    let snap = RealizedVolConeSnapshot {
        symbol: "DDD".into(),
        as_of: "2026-04-15".into(),
        bars_used: 253,
        latest_close: 120.0,
        rv20_pct: 25.0,
        rv60_pct: 22.0,
        rv120_pct: 20.0,
        rv252_pct: 18.0,
        rv20_min_pct: 10.0,
        rv20_median_pct: 20.0,
        rv20_max_pct: 40.0,
        rv20_percentile: 75.0,
        cone_label: "ELEVATED".into(),
        ..Default::default()
    };
    upsert_rvcone(&c, "DDD", &snap).unwrap();
    let got = get_rvcone(&c, "DDD").unwrap().unwrap();
    assert_eq!(got.cone_label, "ELEVATED");
}

#[test]
fn calpb_snapshot_roundtrip() {
    let c = rusqlite::Connection::open_in_memory().unwrap();
    create_research_tables_v21(&c).unwrap();
    let snap = CalendarPeriodBreakdownSnapshot {
        symbol: "EEE".into(),
        as_of: "2026-04-15".into(),
        bars_used: 253,
        latest_close: 100.0,
        mtd_pct: 2.0,
        qtd_pct: 5.0,
        ytd_pct: 8.0,
        prior_quarter_pct: 1.0,
        prior_year_pct: 10.0,
        current_year: "2026".into(),
        current_quarter: "Q2".into(),
        momentum_label: "ACCELERATING".into(),
        ..Default::default()
    };
    upsert_calpb(&c, "EEE", &snap).unwrap();
    let got = get_calpb(&c, "EEE").unwrap().unwrap();
    assert_eq!(got.momentum_label, "ACCELERATING");
}

#[test]
fn compute_betarank_safest_decile() {
    // Subject has the lowest beta by far → safest decile.
    let peers = vec![
        ("B".to_string(), Some(1.5)),
        ("C".to_string(), Some(1.8)),
        ("D".to_string(), Some(2.0)),
        ("E".to_string(), Some(1.6)),
        ("F".to_string(), Some(1.7)),
        ("G".to_string(), Some(1.9)),
        ("H".to_string(), Some(2.1)),
        ("I".to_string(), Some(1.4)),
        ("J".to_string(), Some(1.55)),
        ("K".to_string(), Some(2.2)),
    ];
    let snap = compute_betarank_snapshot("AAA", "2026-04-15", "Technology", Some(0.6), &peers);
    assert_eq!(snap.rank_label, "SAFEST_DECILE");
    assert_eq!(snap.rank_position, 1);
}

#[test]
fn compute_betarank_riskiest_decile() {
    // Subject has the highest beta → riskiest decile.
    let peers = vec![
        ("B".to_string(), Some(0.6)),
        ("C".to_string(), Some(0.7)),
        ("D".to_string(), Some(0.8)),
        ("E".to_string(), Some(0.9)),
        ("F".to_string(), Some(1.0)),
        ("G".to_string(), Some(1.1)),
        ("H".to_string(), Some(1.2)),
        ("I".to_string(), Some(1.3)),
        ("J".to_string(), Some(1.4)),
        ("K".to_string(), Some(1.5)),
    ];
    let snap = compute_betarank_snapshot("AAA", "2026-04-15", "Technology", Some(2.5), &peers);
    assert_eq!(snap.rank_label, "RISKIEST_DECILE");
}

#[test]
fn compute_betarank_insufficient() {
    let peers = vec![("B".to_string(), Some(1.0)), ("C".to_string(), Some(1.1))];
    let snap = compute_betarank_snapshot("AAA", "2026-04-15", "Technology", Some(0.9), &peers);
    assert_eq!(snap.rank_label, "INSUFFICIENT_DATA");
}

#[test]
fn compute_pegrank_top_decile() {
    // Subject has the lowest PEG → top (best value) decile.
    let peers = vec![
        ("B".to_string(), Some(2.0)),
        ("C".to_string(), Some(2.2)),
        ("D".to_string(), Some(2.5)),
        ("E".to_string(), Some(2.8)),
        ("F".to_string(), Some(3.0)),
        ("G".to_string(), Some(3.2)),
        ("H".to_string(), Some(3.5)),
        ("I".to_string(), Some(3.8)),
        ("J".to_string(), Some(4.0)),
        ("K".to_string(), Some(4.5)),
    ];
    let snap = compute_pegrank_snapshot("AAA", "2026-04-15", "Technology", Some(0.5), &peers);
    assert_eq!(snap.rank_label, "TOP_DECILE");
    assert_eq!(snap.rank_position, 1);
}

#[test]
fn compute_pegrank_filters_negative() {
    // Negative/missing peer PEGs get filtered out.
    let peers = vec![
        ("B".to_string(), Some(2.0)),
        ("C".to_string(), Some(2.5)),
        ("D".to_string(), None),
        ("E".to_string(), Some(-1.5)),
        ("F".to_string(), Some(3.0)),
    ];
    let snap = compute_pegrank_snapshot("AAA", "2026-04-15", "Technology", Some(1.5), &peers);
    assert_eq!(snap.peers_with_data, 3);
}

#[test]
fn compute_pegrank_subject_negative() {
    let peers = vec![
        ("B".to_string(), Some(2.0)),
        ("C".to_string(), Some(2.5)),
        ("D".to_string(), Some(3.0)),
    ];
    let snap = compute_pegrank_snapshot("AAA", "2026-04-15", "Technology", Some(-0.5), &peers);
    assert_eq!(snap.rank_label, "NO_DATA");
}

#[test]
fn compute_fhighlow_at_high() {
    // Latest close == the highest close in the window.
    let mut bars = Vec::new();
    for i in 0..253 {
        let date = format!("2025-{:02}-{:02}", (i / 21) + 1, (i % 21) + 1);
        let c = 100.0 + i as f64 * 0.5; // monotone up
        bars.push(mk_hp(&date, c, c + 0.5, c - 0.5, c));
    }
    let snap = compute_fhighlow_snapshot("AAA", "2026-04-15", &bars);
    assert_eq!(snap.proximity_label, "AT_HIGH");
    assert_eq!(snap.days_since_high, 0);
    assert!((snap.pct_from_high - 0.0).abs() < 1e-9);
}

#[test]
fn compute_fhighlow_at_low() {
    // Monotone down → latest close == lowest close.
    let mut bars = Vec::new();
    for i in 0..253 {
        let date = format!("2025-{:02}-{:02}", (i / 21) + 1, (i % 21) + 1);
        let c = 200.0 - i as f64 * 0.5;
        bars.push(mk_hp(&date, c, c + 0.5, c - 0.5, c));
    }
    let snap = compute_fhighlow_snapshot("AAA", "2026-04-15", &bars);
    assert_eq!(snap.proximity_label, "AT_LOW");
    assert_eq!(snap.days_since_low, 0);
}

#[test]
fn compute_fhighlow_insufficient() {
    let bars = vec![mk_hp("2025-06-01", 100.0, 101.0, 99.0, 100.0)];
    let snap = compute_fhighlow_snapshot("AAA", "2026-04-15", &bars);
    assert_eq!(snap.proximity_label, "INSUFFICIENT_DATA");
}

#[test]
fn compute_rvcone_compressed() {
    // Flat-ish series → compressed / below-avg realized vol.
    let mut bars = Vec::new();
    for i in 0..260 {
        let date = format!("2025-{:02}-{:02}", (i / 20) + 1, (i % 20) + 1);
        let c = 100.0 + (i as f64 * 0.0001);
        bars.push(mk_hp(&date, c, c + 0.001, c - 0.001, c));
    }
    let snap = compute_rvcone_snapshot("AAA", "2026-04-15", &bars);
    assert!(snap.cone_label != "INSUFFICIENT_DATA");
    assert!(
        snap.rv252_pct < 5.0,
        "expected low vol, got {}",
        snap.rv252_pct
    );
}

#[test]
fn compute_rvcone_extreme() {
    // Highly variable → elevated / extreme.
    let mut bars = Vec::new();
    for i in 0..260 {
        let date = format!("2025-{:02}-{:02}", (i / 20) + 1, (i % 20) + 1);
        // Large oscillations with bigger swings at the end.
        let base = 100.0;
        let amp = if i < 200 { 1.0 } else { 10.0 };
        let c = base + amp * ((i as f64 * 0.5).sin() * (i as f64 * 0.3).cos());
        bars.push(mk_hp(&date, c, c + 0.5, c - 0.5, c));
    }
    let snap = compute_rvcone_snapshot("AAA", "2026-04-15", &bars);
    assert!(snap.cone_label != "INSUFFICIENT_DATA");
    assert!(
        snap.rv20_pct > snap.rv252_pct,
        "expected recent 20d RV > 252d RV due to amplitude shift, got rv20={} rv252={}",
        snap.rv20_pct,
        snap.rv252_pct
    );
}

#[test]
fn compute_rvcone_insufficient() {
    let bars = vec![mk_hp("2025-06-01", 100.0, 101.0, 99.0, 100.0)];
    let snap = compute_rvcone_snapshot("AAA", "2026-04-15", &bars);
    assert_eq!(snap.cone_label, "INSUFFICIENT_DATA");
}

#[test]
fn compute_calpb_accelerating() {
    // Q1 2026 flat, Q2 2026 big up move — accelerating vs prior quarter.
    let mut bars = Vec::new();
    // Prior year 2025 Q4 (Oct-Dec): bars from 100→100 (flat prior year Q4).
    for m in 10..=12 {
        for d in 1..=20 {
            let c = 100.0;
            bars.push(mk_hp(
                &format!("2025-{:02}-{:02}", m, d),
                c,
                c + 0.1,
                c - 0.1,
                c,
            ));
        }
    }
    // 2026 Q1 (Jan-Mar) flat at 100 → 100.5
    for m in 1..=3 {
        for d in 1..=20 {
            let c = 100.0 + ((m - 1) * 20 + d) as f64 * 0.01;
            bars.push(mk_hp(
                &format!("2026-{:02}-{:02}", m, d),
                c,
                c + 0.1,
                c - 0.1,
                c,
            ));
        }
    }
    // 2026 Q2 (Apr): big up move.
    for d in 1..=15 {
        let c = 100.5 + d as f64 * 1.0;
        bars.push(mk_hp(&format!("2026-04-{:02}", d), c, c + 0.1, c - 0.1, c));
    }
    let snap = compute_calpb_snapshot("AAA", "2026-04-15", &bars);
    assert_eq!(snap.current_year, "2026");
    assert_eq!(snap.current_quarter, "Q2");
    assert!(snap.qtd_pct > 5.0, "expected QTD up, got {}", snap.qtd_pct);
    assert_eq!(snap.momentum_label, "ACCELERATING");
}

#[test]
fn compute_calpb_insufficient() {
    let bars = vec![mk_hp("2026-04-15", 100.0, 101.0, 99.0, 100.0)];
    let snap = compute_calpb_snapshot("AAA", "2026-04-15", &bars);
    assert_eq!(snap.momentum_label, "INSUFFICIENT_DATA");
}

