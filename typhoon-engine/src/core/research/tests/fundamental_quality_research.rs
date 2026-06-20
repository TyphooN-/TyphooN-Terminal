// ── Research section ──

fn open_mem_conn_v10() -> Connection {
    let c = Connection::open_in_memory().unwrap();
    create_research_tables_v10(&c).unwrap();
    c
}

fn sample_statements() -> FinancialStatements {
    // Build 4 quarters of synthetic financials with positive EBITDA + FCF.
    let q = |d: &str, p: &str, ni: f64, ebitda: f64, int_exp: f64, fcf: f64| {
        (
            IncomeStatement {
                date: d.into(),
                period: p.into(),
                revenue: ni * 10.0,
                cost_of_revenue: ni * 5.0,
                gross_profit: ni * 5.0,
                research_and_development: ni * 0.5,
                selling_general_admin: ni * 1.0,
                operating_expenses: ni * 1.5,
                operating_income: ni * 2.0,
                interest_expense: int_exp,
                ebitda,
                income_before_tax: ni * 1.2,
                income_tax_expense: ni * 0.2,
                net_income: ni,
                eps: ni / 1000.0,
                eps_diluted: ni / 1000.0,
                weighted_shares_out: 1000.0,
            },
            CashFlowStatement {
                date: d.into(),
                period: p.into(),
                net_income: ni,
                depreciation_amortization: ebitda - ni * 2.0,
                stock_based_comp: 0.0,
                change_working_capital: 0.0,
                cash_from_operations: fcf + 50.0,
                capex: -50.0,
                acquisitions: 0.0,
                investments_purchases: 0.0,
                cash_from_investing: -50.0,
                debt_repayment: 0.0,
                dividends_paid: -20.0,
                stock_repurchases: 0.0,
                cash_from_financing: -20.0,
                net_change_cash: 10.0,
                free_cash_flow: fcf,
            },
        )
    };
    let periods = [
        ("2024-12-31", "Q4", 300.0, 500.0, 30.0, 280.0),
        ("2024-09-30", "Q3", 280.0, 480.0, 30.0, 260.0),
        ("2024-06-30", "Q2", 260.0, 460.0, 30.0, 240.0),
        ("2024-03-31", "Q1", 240.0, 440.0, 30.0, 220.0),
    ];
    let mut income_q = Vec::new();
    let mut cf_q = Vec::new();
    for (d, p, ni, ebitda, int_exp, fcf) in periods.iter() {
        let (i, c) = q(d, p, *ni, *ebitda, *int_exp, *fcf);
        income_q.push(i);
        cf_q.push(c);
    }
    let bal = BalanceSheet {
        date: "2024-12-31".into(),
        period: "FY".into(),
        cash_and_equiv: 500.0,
        short_term_investments: 0.0,
        net_receivables: 100.0,
        inventory: 200.0,
        total_current_assets: 800.0,
        property_plant_equipment: 1000.0,
        goodwill: 0.0,
        intangible_assets: 0.0,
        long_term_investments: 0.0,
        total_non_current_assets: 1000.0,
        total_assets: 1800.0,
        accounts_payable: 150.0,
        short_term_debt: 100.0,
        total_current_liabilities: 400.0,
        long_term_debt: 600.0,
        total_non_current_liabilities: 800.0,
        total_liabilities: 1200.0,
        common_stock: 200.0,
        retained_earnings: 400.0,
        total_equity: 600.0,
        total_debt: 700.0,
        net_debt: 200.0,
    };
    FinancialStatements {
        income_annual: vec![income_q[0].clone()],
        income_quarterly: income_q,
        balance_annual: vec![bal.clone()],
        balance_quarterly: vec![bal],
        cashflow_annual: vec![cf_q[0].clone()],
        cashflow_quarterly: cf_q,
    }
}

#[test]
fn leverage_snapshot_roundtrip() {
    let c = open_mem_conn_v10();
    let snap = LeverageSnapshot {
        symbol: "AAPL".into(),
        as_of: "2026-04-14".into(),
        total_debt: 700.0,
        net_debt: 200.0,
        ebitda_ttm: 1880.0,
        interest_expense_ttm: 120.0,
        total_equity: 600.0,
        ratios: vec![LeverageRatio {
            name: "Debt / EBITDA".into(),
            value: 0.37,
            peer_median: 0.0,
            signal: "HEALTHY".into(),
            note: "".into(),
        }],
        solvency_summary: "HEALTHY".into(),
        note: "".into(),
    };
    upsert_leverage(&c, "AAPL", &snap).unwrap();
    let got = get_leverage(&c, "aapl").unwrap().unwrap();
    assert_eq!(got.symbol, "AAPL");
    assert_eq!(got.ratios.len(), 1);
    assert_eq!(got.solvency_summary, "HEALTHY");
}

#[test]
fn accruals_snapshot_roundtrip() {
    let c = open_mem_conn_v10();
    let snap = AccrualsSnapshot {
        symbol: "MSFT".into(),
        as_of: "2026-04-14".into(),
        ttm_net_income: 1000.0,
        ttm_free_cash_flow: 900.0,
        ttm_cash_conversion_pct: 90.0,
        avg_cash_conversion_pct: 85.0,
        periods: vec![],
        trend_label: "STABLE".into(),
        note: "".into(),
    };
    upsert_accruals(&c, "MSFT", &snap).unwrap();
    let got = get_accruals(&c, "msft").unwrap().unwrap();
    assert_eq!(got.ttm_cash_conversion_pct, 90.0);
}

#[test]
fn realized_vol_snapshot_roundtrip() {
    let c = open_mem_conn_v10();
    let snap = RealizedVolSnapshot {
        symbol: "NVDA".into(),
        as_of: "2026-04-14".into(),
        last_close: 900.0,
        current_atm_iv_pct: 45.0,
        iv_rv_gap_pct: 10.0,
        iv_rv_ratio: 1.28,
        windows: vec![RealizedVolWindow {
            label: "20d".into(),
            trading_days: 20,
            realized_vol_pct: 35.0,
            percentile: 60.0,
            n_observations: 100,
        }],
        regime_label: "RICH_IV".into(),
        note: "".into(),
    };
    upsert_realized_vol(&c, "NVDA", &snap).unwrap();
    let got = get_realized_vol(&c, "nvda").unwrap().unwrap();
    assert_eq!(got.windows.len(), 1);
    assert_eq!(got.regime_label, "RICH_IV");
}

#[test]
fn fcf_yield_snapshot_roundtrip() {
    let c = open_mem_conn_v10();
    let snap = FcfYieldSnapshot {
        symbol: "KO".into(),
        as_of: "2026-04-14".into(),
        market_cap: 300_000_000_000.0,
        ttm_free_cash_flow: 10_000_000_000.0,
        ttm_dividends_paid: 8_000_000_000.0,
        ttm_fcf_yield_pct: 3.33,
        ttm_dividend_yield_pct: 2.67,
        ttm_payout_from_fcf_pct: 80.0,
        ttm_payout_from_ni_pct: 70.0,
        fcf_cagr_5y_pct: 5.2,
        periods: vec![],
        sustainability_label: "STRETCHED".into(),
        note: "".into(),
    };
    upsert_fcf_yield(&c, "KO", &snap).unwrap();
    let got = get_fcf_yield(&c, "ko").unwrap().unwrap();
    assert_eq!(got.sustainability_label, "STRETCHED");
}

#[test]
fn short_interest_snapshot_roundtrip() {
    let c = open_mem_conn_v10();
    let snap = ShortInterestSnapshot {
        symbol: "GME".into(),
        as_of: "2026-04-14".into(),
        shares_outstanding: 300_000_000.0,
        shares_float: 200_000_000.0,
        short_shares: 50_000_000.0,
        short_percent_of_float: 25.0,
        avg_daily_volume_20d: 5_000_000.0,
        days_to_cover: 10.0,
        short_ratio_reported: 0.0,
        utilization_proxy_pct: 25.0,
        squeeze_risk_label: "EXTREME".into(),
        note: "".into(),
    };
    upsert_short_interest(&c, "GME", &snap).unwrap();
    let got = get_short_interest(&c, "gme").unwrap().unwrap();
    assert_eq!(got.squeeze_risk_label, "EXTREME");
    assert_eq!(got.days_to_cover, 10.0);
}

#[test]
fn compute_leverage_on_healthy_statements() {
    let st = sample_statements();
    let snap = compute_leverage_snapshot("TEST", "2026-04-14", &st, 700.0, 500.0);
    // TTM EBITDA = 500+480+460+440 = 1880 → Debt/EBITDA = 700/1880 ≈ 0.37.
    assert!(!snap.ratios.is_empty());
    let de = snap
        .ratios
        .iter()
        .find(|r| r.name == "Debt / EBITDA")
        .unwrap();
    assert!((de.value - 700.0 / 1880.0).abs() < 1e-6);
    assert_eq!(de.signal, "HEALTHY");
    assert_eq!(snap.ebitda_ttm, 1880.0);
}

#[test]
fn compute_leverage_empty_statements_produces_note() {
    let st = FinancialStatements::default();
    let snap = compute_leverage_snapshot("X", "2026-04-14", &st, 0.0, 0.0);
    assert!(snap.ratios.is_empty());
    assert!(!snap.note.is_empty());
}

#[test]
fn compute_accruals_high_conversion_labels_high() {
    let st = sample_statements();
    let snap = compute_accruals_snapshot("TEST", "2026-04-14", &st);
    // Q4: NI=300, FCF=280 → conv=93.3% → HIGH.
    assert_eq!(snap.periods.len(), 4);
    let latest = &snap.periods[0];
    assert_eq!(latest.quality_label, "HIGH");
    let ttm_ni: f64 = 300.0 + 280.0 + 260.0 + 240.0;
    assert!((snap.ttm_net_income - ttm_ni).abs() < 1e-6);
}

#[test]
fn compute_accruals_insufficient_periods_labels_insufficient() {
    let mut st = FinancialStatements::default();
    st.income_quarterly.push(IncomeStatement {
        date: "2024-12-31".into(),
        period: "Q4".into(),
        net_income: 100.0,
        ..Default::default()
    });
    st.cashflow_quarterly.push(CashFlowStatement {
        date: "2024-12-31".into(),
        period: "Q4".into(),
        net_income: 100.0,
        free_cash_flow: 90.0,
        ..Default::default()
    });
    let snap = compute_accruals_snapshot("X", "2026-04-14", &st);
    assert_eq!(snap.periods.len(), 1);
    assert_eq!(snap.trend_label, "INSUFFICIENT");
}

#[test]
fn compute_realized_vol_with_drift_produces_rich_regime() {
    let bars = synth_bars(260, 100.0, 0.001);
    let snap = compute_realized_vol_snapshot("TEST", "2026-04-14", &bars, 40.0);
    assert!(!snap.windows.is_empty());
    assert!(snap.windows.iter().any(|w| w.label == "20d"));
    // Constant drift → near-zero RV → IV/RV should flag RICH_IV or NO_IV_REFERENCE.
    assert!(snap.regime_label == "RICH_IV" || snap.regime_label == "NO_IV_REFERENCE");
}

#[test]
fn compute_realized_vol_insufficient_bars_returns_note() {
    let bars = synth_bars(10, 100.0, 0.001);
    let snap = compute_realized_vol_snapshot("X", "2026-04-14", &bars, 40.0);
    assert_eq!(snap.regime_label, "INSUFFICIENT_DATA");
    assert!(!snap.note.is_empty());
}

#[test]
fn compute_fcf_yield_with_market_cap() {
    let st = sample_statements();
    let snap = compute_fcf_yield_snapshot("TEST", "2026-04-14", &st, 100_000.0, 100.0);
    // TTM FCF = 280+260+240+220 = 1000; yield = 1000/100000 = 1.0%
    assert!((snap.ttm_free_cash_flow - 1000.0).abs() < 1e-6);
    assert!((snap.ttm_fcf_yield_pct - 1.0).abs() < 1e-6);
    // TTM dividends paid = 20*4 = 80, payout_fcf = 80/1000 = 8%, label SAFE.
    assert_eq!(snap.sustainability_label, "SAFE");
}

#[test]
fn compute_fcf_yield_no_market_cap_emits_note() {
    let st = sample_statements();
    let snap = compute_fcf_yield_snapshot("X", "2026-04-14", &st, 0.0, 100.0);
    assert!(!snap.note.is_empty());
}

#[test]
fn compute_short_interest_high_risk_squeeze() {
    let bars = synth_bars(30, 100.0, 0.0);
    // 200M float × 25% = 50M short; 50M / 1K avg = 50K days-to-cover → EXTREME.
    let snap = compute_short_interest_snapshot(
        "GME",
        "2026-04-14",
        300_000_000.0,
        200_000_000.0,
        25.0,
        0.0,
        &bars,
    );
    assert_eq!(snap.short_shares, 50_000_000.0);
    assert_eq!(snap.squeeze_risk_label, "EXTREME");
}

#[test]
fn compute_short_interest_no_shorts_insufficient() {
    let bars = synth_bars(30, 100.0, 0.0);
    let snap = compute_short_interest_snapshot(
        "X",
        "2026-04-14",
        100_000_000.0,
        80_000_000.0,
        0.0,
        0.0,
        &bars,
    );
    assert_eq!(snap.squeeze_risk_label, "INSUFFICIENT_DATA");
}

// Fundamental quality and solvency tests

fn open_mem_conn_v11() -> Connection {
    let c = Connection::open_in_memory().unwrap();
    create_research_tables_v11(&c).unwrap();
    c
}

fn two_year_statements() -> FinancialStatements {
    // Build two annual snapshots so Piotroski and Altman have enough data.
    let inc_current = IncomeStatement {
        date: "2024-12-31".into(),
        period: "FY".into(),
        revenue: 2000.0,
        cost_of_revenue: 1000.0,
        gross_profit: 1000.0,
        research_and_development: 100.0,
        selling_general_admin: 200.0,
        operating_expenses: 300.0,
        operating_income: 700.0,
        interest_expense: 50.0,
        ebitda: 800.0,
        income_before_tax: 650.0,
        income_tax_expense: 150.0,
        net_income: 500.0,
        eps: 5.0,
        eps_diluted: 5.0,
        weighted_shares_out: 100.0,
    };
    let inc_prior = IncomeStatement {
        date: "2023-12-31".into(),
        period: "FY".into(),
        revenue: 1800.0,
        cost_of_revenue: 1000.0,
        gross_profit: 800.0,
        research_and_development: 100.0,
        selling_general_admin: 200.0,
        operating_expenses: 300.0,
        operating_income: 500.0,
        interest_expense: 50.0,
        ebitda: 600.0,
        income_before_tax: 450.0,
        income_tax_expense: 100.0,
        net_income: 350.0,
        eps: 3.5,
        eps_diluted: 3.5,
        weighted_shares_out: 100.0,
    };
    let bal_current = BalanceSheet {
        date: "2024-12-31".into(),
        period: "FY".into(),
        cash_and_equiv: 400.0,
        short_term_investments: 0.0,
        net_receivables: 200.0,
        inventory: 300.0,
        total_current_assets: 900.0,
        property_plant_equipment: 1500.0,
        goodwill: 0.0,
        intangible_assets: 0.0,
        long_term_investments: 0.0,
        total_non_current_assets: 1500.0,
        total_assets: 2400.0,
        accounts_payable: 150.0,
        short_term_debt: 100.0,
        total_current_liabilities: 400.0,
        long_term_debt: 500.0,
        total_non_current_liabilities: 700.0,
        total_liabilities: 1100.0,
        common_stock: 300.0,
        retained_earnings: 1000.0,
        total_equity: 1300.0,
        total_debt: 600.0,
        net_debt: 200.0,
    };
    let bal_prior = BalanceSheet {
        date: "2023-12-31".into(),
        period: "FY".into(),
        cash_and_equiv: 300.0,
        short_term_investments: 0.0,
        net_receivables: 180.0,
        inventory: 280.0,
        total_current_assets: 760.0,
        property_plant_equipment: 1400.0,
        goodwill: 0.0,
        intangible_assets: 0.0,
        long_term_investments: 0.0,
        total_non_current_assets: 1400.0,
        total_assets: 2160.0,
        accounts_payable: 150.0,
        short_term_debt: 100.0,
        total_current_liabilities: 400.0,
        long_term_debt: 600.0,
        total_non_current_liabilities: 800.0,
        total_liabilities: 1200.0,
        common_stock: 300.0,
        retained_earnings: 660.0,
        total_equity: 960.0,
        total_debt: 700.0,
        net_debt: 400.0,
    };
    let cf_current = CashFlowStatement {
        date: "2024-12-31".into(),
        period: "FY".into(),
        net_income: 500.0,
        depreciation_amortization: 100.0,
        stock_based_comp: 0.0,
        change_working_capital: 0.0,
        cash_from_operations: 600.0,
        capex: -200.0,
        acquisitions: 0.0,
        investments_purchases: 0.0,
        cash_from_investing: -200.0,
        debt_repayment: -100.0,
        dividends_paid: 0.0,
        stock_repurchases: 0.0,
        cash_from_financing: -100.0,
        net_change_cash: 300.0,
        free_cash_flow: 400.0,
    };
    let cf_prior = CashFlowStatement {
        date: "2023-12-31".into(),
        period: "FY".into(),
        net_income: 350.0,
        depreciation_amortization: 90.0,
        stock_based_comp: 0.0,
        change_working_capital: 0.0,
        cash_from_operations: 420.0,
        capex: -180.0,
        acquisitions: 0.0,
        investments_purchases: 0.0,
        cash_from_investing: -180.0,
        debt_repayment: 0.0,
        dividends_paid: 0.0,
        stock_repurchases: 0.0,
        cash_from_financing: 0.0,
        net_change_cash: 240.0,
        free_cash_flow: 240.0,
    };
    FinancialStatements {
        income_annual: vec![inc_current.clone(), inc_prior.clone()],
        income_quarterly: vec![inc_current],
        balance_annual: vec![bal_current.clone(), bal_prior.clone()],
        balance_quarterly: vec![bal_current],
        cashflow_annual: vec![cf_current.clone(), cf_prior.clone()],
        cashflow_quarterly: vec![cf_current],
    }
}

#[test]
fn altman_z_snapshot_roundtrip() {
    let c = open_mem_conn_v11();
    let snap = AltmanZSnapshot {
        symbol: "AAPL".into(),
        as_of: "2026-04-14".into(),
        working_capital: 500.0,
        retained_earnings: 1000.0,
        ebit: 700.0,
        market_value_equity: 5000.0,
        sales: 2000.0,
        total_assets: 2400.0,
        total_liabilities: 1100.0,
        z_score: 4.2,
        zone: "SAFE".into(),
        components: vec![AltmanComponent {
            name: "A: WC/TA".into(),
            ratio: 0.208,
            coefficient: 1.2,
            contribution: 0.25,
            note: "".into(),
        }],
        note: "".into(),
    };
    upsert_altman_z(&c, "AAPL", &snap).unwrap();
    let got = get_altman_z(&c, "aapl").unwrap().unwrap();
    assert_eq!(got.symbol, "AAPL");
    assert_eq!(got.zone, "SAFE");
    assert_eq!(got.components.len(), 1);
}

#[test]
fn piotroski_snapshot_roundtrip() {
    let c = open_mem_conn_v11();
    let snap = PiotroskiSnapshot {
        symbol: "MSFT".into(),
        as_of: "2026-04-14".into(),
        current_period: "2024-12-31".into(),
        prior_period: "2023-12-31".into(),
        f_score: 8,
        strength_label: "STRONG".into(),
        profitability_score: 4,
        leverage_score: 2,
        efficiency_score: 2,
        checks: vec![PiotroskiCheck {
            category: "Profitability".into(),
            name: "Positive Net Income".into(),
            passed: true,
            value_current: 500.0,
            value_prior: 350.0,
            note: "".into(),
        }],
        note: "".into(),
    };
    upsert_piotroski(&c, "MSFT", &snap).unwrap();
    let got = get_piotroski(&c, "msft").unwrap().unwrap();
    assert_eq!(got.f_score, 8);
    assert_eq!(got.strength_label, "STRONG");
}

#[test]
fn ohlc_vol_snapshot_roundtrip() {
    let c = open_mem_conn_v11();
    let snap = OhlcVolSnapshot {
        symbol: "NVDA".into(),
        as_of: "2026-04-14".into(),
        trading_days: 60,
        estimators: vec![VolEstimator {
            name: "YangZhang".into(),
            annualized_vol_pct: 35.0,
            efficiency_vs_close: 1.1,
            note: "".into(),
        }],
        preferred_estimate_pct: 35.0,
        preferred_label: "YangZhang".into(),
        note: "".into(),
    };
    upsert_ohlc_vol(&c, "NVDA", &snap).unwrap();
    let got = get_ohlc_vol(&c, "nvda").unwrap().unwrap();
    assert_eq!(got.preferred_label, "YangZhang");
    assert_eq!(got.estimators.len(), 1);
}

#[test]
fn eps_beat_snapshot_roundtrip() {
    let c = open_mem_conn_v11();
    let snap = EpsBeatSnapshot {
        symbol: "AMZN".into(),
        as_of: "2026-04-14".into(),
        total_reports: 8,
        beats: 6,
        misses: 1,
        inlines: 1,
        beat_rate_pct: 75.0,
        current_streak: 3,
        longest_beat_streak: 4,
        longest_miss_streak: 1,
        avg_surprise_pct: 5.2,
        median_surprise_pct: 4.8,
        recent_avg_surprise_pct: 6.5,
        bias_label: "POSITIVE".into(),
        trend_label: "ACCELERATING".into(),
        latest_date: "2024-10-31".into(),
        latest_surprise_pct: 7.0,
        note: "".into(),
    };
    upsert_eps_beat(&c, "AMZN", &snap).unwrap();
    let got = get_eps_beat(&c, "amzn").unwrap().unwrap();
    assert_eq!(got.beats, 6);
    assert_eq!(got.trend_label, "ACCELERATING");
}

#[test]
fn price_target_dispersion_roundtrip() {
    let c = open_mem_conn_v11();
    let snap = PriceTargetDispersion {
        symbol: "TSLA".into(),
        as_of: "2026-04-14".into(),
        current_price: 200.0,
        target_high: 350.0,
        target_low: 150.0,
        target_mean: 250.0,
        target_median: 240.0,
        num_analysts: 25,
        dispersion_pct: 80.0,
        spread_pct: 100.0,
        implied_return_median_pct: 20.0,
        implied_return_mean_pct: 25.0,
        upside_to_high_pct: 75.0,
        downside_to_low_pct: -25.0,
        consensus_label: "BULLISH".into(),
        note: "".into(),
    };
    upsert_price_target_dispersion(&c, "TSLA", &snap).unwrap();
    let got = get_price_target_dispersion(&c, "tsla").unwrap().unwrap();
    assert_eq!(got.consensus_label, "BULLISH");
    assert_eq!(got.num_analysts, 25);
}

#[test]
fn compute_altman_z_on_healthy_statements() {
    let st = two_year_statements();
    let snap = compute_altman_z_snapshot("TEST", "2026-04-14", &st, 5000.0);
    // WC = 900-400 = 500, TA = 2400, RE = 1000, EBIT = 700, MVE = 5000, TL = 1100, Sales = 2000
    // Z = 1.2*(500/2400) + 1.4*(1000/2400) + 3.3*(700/2400) + 0.6*(5000/1100) + 1.0*(2000/2400)
    //   ≈ 0.25 + 0.583 + 0.963 + 2.727 + 0.833 ≈ 5.36 → SAFE
    assert_eq!(snap.components.len(), 5);
    assert!(snap.z_score > 2.99);
    assert_eq!(snap.zone, "SAFE");
    assert_eq!(snap.total_assets, 2400.0);
}

#[test]
fn compute_altman_z_insufficient_data_returns_note() {
    let st = FinancialStatements::default();
    let snap = compute_altman_z_snapshot("X", "2026-04-14", &st, 1000.0);
    assert_eq!(snap.zone, "INSUFFICIENT_DATA");
    assert!(!snap.note.is_empty());
}

#[test]
fn compute_piotroski_strong_score() {
    let st = two_year_statements();
    let snap = compute_piotroski_snapshot("TEST", "2026-04-14", &st);
    // Improving NI (350→500), positive OCF (600), OCF>NI, LTDebt↓ (600→500),
    // current ratio ≈ 900/400 vs 760/400 → improved, no new shares, GM ≈ 50% vs 44% → improved,
    // asset turnover ≈ 2000/2400 vs 1800/2160 → similar, expect STRONG (≥7).
    assert!(snap.f_score >= 7);
    assert_eq!(snap.strength_label, "STRONG");
    assert_eq!(snap.checks.len(), 9);
}

#[test]
fn compute_piotroski_insufficient_data() {
    let st = FinancialStatements::default();
    let snap = compute_piotroski_snapshot("X", "2026-04-14", &st);
    assert_eq!(snap.strength_label, "INSUFFICIENT_DATA");
    assert!(!snap.note.is_empty());
}

#[test]
fn compute_ohlc_vol_five_estimators() {
    let bars = synth_bars(60, 100.0, 0.001);
    let snap = compute_ohlc_vol_snapshot("TEST", "2026-04-14", &bars, 30);
    assert_eq!(snap.estimators.len(), 5);
    assert!(snap.preferred_estimate_pct >= 0.0);
    assert_eq!(snap.preferred_label, "Yang-Zhang");
}

#[test]
fn compute_ohlc_vol_insufficient_bars() {
    let bars = synth_bars(10, 100.0, 0.001);
    let snap = compute_ohlc_vol_snapshot("X", "2026-04-14", &bars, 20);
    assert!(!snap.note.is_empty());
    assert!(snap.estimators.is_empty());
}

#[test]
fn compute_eps_beat_six_beats_labels_positive() {
    let rows: Vec<EarningsSurprise> = (0..8)
        .map(|i| EarningsSurprise {
            date: format!("2024-{:02}-01", i + 1),
            symbol: "TEST".into(),
            eps_actual: 1.0 + (i as f64) * 0.05,
            eps_estimate: 1.0,
            surprise: (i as f64) * 0.05,
            surprise_pct: (i as f64) * 5.0,
        })
        .collect();
    let snap = compute_eps_beat_snapshot("TEST", "2026-04-14", &rows);
    assert_eq!(snap.total_reports, 8);
    assert!(snap.beats >= 6);
    assert_eq!(snap.bias_label, "POSITIVE");
    assert!(snap.current_streak > 0);
}

#[test]
fn compute_eps_beat_empty_reports() {
    let snap = compute_eps_beat_snapshot("X", "2026-04-14", &[]);
    assert_eq!(snap.total_reports, 0);
    assert!(!snap.note.is_empty());
}

#[test]
fn compute_price_target_dispersion_bullish() {
    let target = PriceTarget {
        symbol: "TEST".into(),
        target_high: 150.0,
        target_low: 110.0,
        target_mean: 130.0,
        target_median: 125.0,
        last_updated: "2024-11-01".into(),
        num_analysts: 15,
    };
    let snap = compute_price_target_dispersion("TEST", "2026-04-14", 100.0, Some(&target));
    // implied_median = (125-100)/100 = 25% → BULLISH
    assert_eq!(snap.consensus_label, "BULLISH");
    assert!((snap.implied_return_median_pct - 25.0).abs() < 1e-6);
    assert!((snap.spread_pct - 40.0).abs() < 1e-6);
}

#[test]
fn compute_price_target_dispersion_no_coverage() {
    let snap = compute_price_target_dispersion("X", "2026-04-14", 100.0, None);
    assert_eq!(snap.consensus_label, "NO_COVERAGE");
    assert!(!snap.note.is_empty());
}

// Insider, dividend, earnings-revision, and sector-rotation tests

fn open_mem_conn_v12() -> Connection {
    let c = Connection::open_in_memory().unwrap();
    create_research_tables_v12(&c).unwrap();
    c
}

fn trade(date: &str, name: &str, ttype: &str, disp: &str, shares: f64, price: f64) -> InsiderTrade {
    InsiderTrade {
        filing_date: date.to_string(),
        transaction_date: date.to_string(),
        reporting_name: name.to_string(),
        transaction_type: ttype.to_string(),
        acquisition_disposition: disp.to_string(),
        shares,
        price,
        value_usd: shares * price,
        shares_owned_after: 0.0,
        link: String::new(),
    }
}

#[test]
fn insider_activity_snapshot_roundtrip() {
    let c = open_mem_conn_v12();
    let snap = InsiderActivitySnapshot {
        symbol: "AAPL".into(),
        as_of: "2026-04-14".into(),
        window_days: 90,
        total_trades: 5,
        buy_count: 3,
        sell_count: 2,
        other_count: 0,
        unique_insiders: 2,
        gross_buy_value_usd: 1_000_000.0,
        gross_sell_value_usd: 400_000.0,
        net_value_usd: 600_000.0,
        buy_sell_ratio: 1.5,
        net_shares: 5_000.0,
        latest_trade_date: "2026-04-10".into(),
        bias_label: "BULLISH".into(),
        conviction_label: "MEDIUM".into(),
        note: String::new(),
    };
    upsert_insider_activity(&c, "AAPL", &snap).unwrap();
    let got = get_insider_activity(&c, "aapl").unwrap().unwrap();
    assert_eq!(got.bias_label, "BULLISH");
    assert_eq!(got.buy_count, 3);
    assert!((got.buy_sell_ratio - 1.5).abs() < 1e-6);
}

#[test]
fn compute_insider_activity_bullish_net_buys() {
    let trades = vec![
        trade("2026-04-10", "Alice CEO", "P-Purchase", "A", 1000.0, 150.0),
        trade("2026-04-05", "Bob CFO", "P-Purchase", "A", 2000.0, 148.0),
        trade("2026-03-20", "Alice CEO", "P-Purchase", "A", 500.0, 145.0),
        trade("2026-03-10", "Carol COO", "S-Sale", "D", 500.0, 160.0),
    ];
    let snap = compute_insider_activity_snapshot("AAPL", "2026-04-14", &trades, 90);
    assert_eq!(snap.bias_label, "BULLISH");
    assert_eq!(snap.buy_count, 3);
    assert_eq!(snap.sell_count, 1);
    assert_eq!(snap.unique_insiders, 3);
    assert!(snap.gross_buy_value_usd > snap.gross_sell_value_usd);
    assert!(snap.net_value_usd > 0.0);
    assert_eq!(snap.latest_trade_date, "2026-04-10");
}

#[test]
fn compute_insider_activity_bearish_net_sales() {
    let trades = vec![
        trade("2026-04-10", "Alice CEO", "S-Sale", "D", 10_000.0, 150.0),
        trade("2026-04-05", "Bob CFO", "S-Sale", "D", 5_000.0, 148.0),
        trade("2026-03-10", "Alice CEO", "P-Purchase", "A", 100.0, 145.0),
    ];
    let snap = compute_insider_activity_snapshot("AAPL", "2026-04-14", &trades, 90);
    assert_eq!(snap.bias_label, "BEARISH");
    assert_eq!(snap.sell_count, 2);
    assert!(snap.net_value_usd < 0.0);
}

#[test]
fn compute_insider_activity_no_activity() {
    let snap = compute_insider_activity_snapshot("X", "2026-04-14", &[], 90);
    assert_eq!(snap.bias_label, "NO_ACTIVITY");
    assert_eq!(snap.conviction_label, "NONE");
    assert!(!snap.note.is_empty());
}

#[test]
fn insider_activity_respects_lookback_window() {
    let trades = vec![
        trade("2026-04-01", "Alice CEO", "P-Purchase", "A", 1000.0, 150.0),
        trade("2025-01-01", "Old CEO", "P-Purchase", "A", 9999.0, 100.0),
    ];
    // 90-day window from 2026-04-14 should exclude 2025-01-01
    let snap = compute_insider_activity_snapshot("X", "2026-04-14", &trades, 90);
    assert_eq!(snap.total_trades, 1);
    assert_eq!(snap.buy_count, 1);
}

fn dvd(ex: &str, amt: f64) -> DividendRecord {
    DividendRecord {
        ex_date: ex.to_string(),
        amount: amt,
        ..Default::default()
    }
}

#[test]
fn divg_snapshot_roundtrip() {
    let c = open_mem_conn_v12();
    let snap = DivgSnapshot {
        symbol: "KO".into(),
        as_of: "2026-04-14".into(),
        total_payments: 20,
        latest_amount: 0.49,
        annualized_dividend: 1.96,
        years_covered: 5,
        cagr_1y_pct: 5.0,
        cagr_3y_pct: 4.5,
        cagr_5y_pct: 4.0,
        trend_label: "GROWING".into(),
        ..Default::default()
    };
    upsert_divg(&c, "KO", &snap).unwrap();
    let got = get_divg(&c, "ko").unwrap().unwrap();
    assert_eq!(got.trend_label, "GROWING");
    assert!((got.cagr_1y_pct - 5.0).abs() < 1e-6);
}

#[test]
fn compute_divg_growing_consistent() {
    // Five complete years of 4 payments each, 5 % YoY growth, last payment before as_of.
    let mut rows = Vec::new();
    for (y, base) in [
        (2020, 0.40),
        (2021, 0.42),
        (2022, 0.44),
        (2023, 0.46),
        (2024, 0.49),
    ] {
        for q in ["03-15", "06-15", "09-15", "12-15"] {
            rows.push(dvd(&format!("{y}-{q}"), base));
        }
    }
    let snap = compute_divg_snapshot("KO", "2026-04-14", &rows);
    assert_eq!(snap.trend_label, "GROWING");
    assert!(snap.years_covered >= 5);
    assert!(snap.cagr_1y_pct > 0.0);
    assert!(snap.consecutive_growth_years >= 4);
    assert!(snap.consistency_score_pct >= 70.0);
}

#[test]
fn compute_divg_cutting() {
    let rows = vec![
        dvd("2022-03-15", 0.80),
        dvd("2022-06-15", 0.80),
        dvd("2022-09-15", 0.80),
        dvd("2022-12-15", 0.80),
        dvd("2023-03-15", 0.70),
        dvd("2023-06-15", 0.60),
        dvd("2023-09-15", 0.50),
        dvd("2023-12-15", 0.40),
    ];
    let snap = compute_divg_snapshot("X", "2026-04-14", &rows);
    assert_eq!(snap.trend_label, "CUTTING");
    assert!(snap.cagr_1y_pct < 0.0);
}

#[test]
fn compute_divg_no_history() {
    let snap = compute_divg_snapshot("X", "2026-04-14", &[]);
    assert_eq!(snap.trend_label, "NO_HISTORY");
}

fn inc(date: &str, rev: f64, eps: f64) -> IncomeStatement {
    IncomeStatement {
        date: date.to_string(),
        period: "Q".to_string(),
        revenue: rev,
        eps,
        ..Default::default()
    }
}

fn surp(date: &str, actual: f64, est: f64) -> EarningsSurprise {
    let s = actual - est;
    EarningsSurprise {
        date: date.to_string(),
        symbol: "X".into(),
        eps_actual: actual,
        eps_estimate: est,
        surprise: s,
        surprise_pct: if est.abs() > 0.0 {
            s / est.abs() * 100.0
        } else {
            0.0
        },
    }
}

#[test]
fn earm_snapshot_roundtrip() {
    let c = open_mem_conn_v12();
    let snap = EarmSnapshot {
        symbol: "NVDA".into(),
        as_of: "2026-04-14".into(),
        quarters_used: 8,
        recent_revenue_growth_pct: 40.0,
        prior_revenue_growth_pct: 25.0,
        revenue_acceleration_pct: 15.0,
        recent_eps_surprise_pct: 10.0,
        prior_eps_surprise_pct: 5.0,
        eps_surprise_acceleration_pct: 5.0,
        composite_score: 80.0,
        momentum_label: "ACCELERATING".into(),
        quarters: vec![EarmQuarterRow {
            period: "2026-01-31".into(),
            revenue: 22000.0,
            revenue_yoy_pct: 40.0,
            eps_actual: 5.0,
            eps_estimate: 4.5,
            eps_surprise_pct: 11.1,
        }],
        note: String::new(),
    };
    upsert_earm(&c, "NVDA", &snap).unwrap();
    let got = get_earm(&c, "nvda").unwrap().unwrap();
    assert_eq!(got.momentum_label, "ACCELERATING");
    assert!((got.composite_score - 80.0).abs() < 1e-6);
}

#[test]
fn compute_earm_accelerating() {
    // 8 quarters, newest-first. Revenue grows yoy, recent pace faster than prior.
    let statements = FinancialStatements {
        income_quarterly: vec![
            inc("2026-03-31", 140.0, 2.40), // 0  yoy vs 4 = +16.67%
            inc("2025-12-31", 135.0, 2.30), // 1  yoy vs 5 = +17.39%
            inc("2025-09-30", 130.0, 2.20), // 2  yoy vs 6 = +18.18%
            inc("2025-06-30", 125.0, 2.10), // 3  yoy vs 7 = +19.05%
            inc("2025-03-31", 120.0, 2.00), // 4
            inc("2024-12-31", 115.0, 1.90), // 5
            inc("2024-09-30", 110.0, 1.80), // 6
            inc("2024-06-30", 105.0, 1.75), // 7
        ],
        ..Default::default()
    };
    let surprises = vec![
        surp("2026-03-31", 2.40, 2.30),
        surp("2025-12-31", 2.30, 2.20),
        surp("2025-09-30", 2.20, 2.15),
        surp("2025-06-30", 2.10, 2.08),
        surp("2025-03-31", 2.00, 1.99),
        surp("2024-12-31", 1.90, 1.90),
        surp("2024-09-30", 1.80, 1.81),
        surp("2024-06-30", 1.75, 1.77),
    ];
    let snap = compute_earm_snapshot("NVDA", "2026-04-14", &statements, &surprises);
    assert_eq!(snap.quarters_used, 8);
    assert!(snap.recent_revenue_growth_pct > 0.0);
    assert!(snap.composite_score > 0.0);
    assert!(matches!(
        snap.momentum_label.as_str(),
        "ACCELERATING" | "STABLE"
    ));
}

#[test]
fn compute_earm_insufficient_data() {
    let statements = FinancialStatements {
        income_quarterly: vec![inc("2026-03-31", 100.0, 1.0)],
        ..Default::default()
    };
    let snap = compute_earm_snapshot("X", "2026-04-14", &statements, &[]);
    assert_eq!(snap.momentum_label, "INSUFFICIENT_DATA");
}

#[test]
fn sector_rotation_snapshot_roundtrip() {
    let c = open_mem_conn_v12();
    let snap = SectorRotationSnapshot {
        symbol: "AAPL".into(),
        as_of: "2026-04-14".into(),
        symbol_sector: "Technology".into(),
        symbol_sector_change_pct: 1.5,
        sector_rank: 1,
        sectors_total: 11,
        avg_sector_change_pct: 0.3,
        relative_strength_pct: 1.2,
        strength_label: "LEADER".into(),
        ..Default::default()
    };
    upsert_sector_rotation(&c, "AAPL", &snap).unwrap();
    let got = get_sector_rotation(&c, "aapl").unwrap().unwrap();
    assert_eq!(got.strength_label, "LEADER");
    assert_eq!(got.sector_rank, 1);
}

#[test]
fn compute_sector_rotation_leader() {
    let sectors = vec![
        SectorPerformance {
            sector: "Technology".into(),
            change_pct: 2.0,
        },
        SectorPerformance {
            sector: "Healthcare".into(),
            change_pct: 0.5,
        },
        SectorPerformance {
            sector: "Financial Services".into(),
            change_pct: 0.1,
        },
        SectorPerformance {
            sector: "Energy".into(),
            change_pct: -0.5,
        },
        SectorPerformance {
            sector: "Consumer Cyclical".into(),
            change_pct: 1.0,
        },
        SectorPerformance {
            sector: "Utilities".into(),
            change_pct: -1.0,
        },
    ];
    let snap = compute_sector_rotation_snapshot("AAPL", "2026-04-14", "Technology", &sectors);
    assert_eq!(snap.strength_label, "LEADER");
    assert_eq!(snap.sector_rank, 1);
    assert_eq!(snap.strongest_sector, "Technology");
    assert_eq!(snap.weakest_sector, "Utilities");
    assert!(snap.relative_strength_pct > 0.0);
}

#[test]
fn compute_sector_rotation_laggard() {
    let sectors = vec![
        SectorPerformance {
            sector: "Technology".into(),
            change_pct: 2.0,
        },
        SectorPerformance {
            sector: "Healthcare".into(),
            change_pct: 1.5,
        },
        SectorPerformance {
            sector: "Financials".into(),
            change_pct: 1.0,
        },
        SectorPerformance {
            sector: "Energy".into(),
            change_pct: -1.5,
        },
    ];
    let snap = compute_sector_rotation_snapshot("XOM", "2026-04-14", "Energy", &sectors);
    assert_eq!(snap.strength_label, "LAGGARD");
    assert!(snap.relative_strength_pct < 0.0);
}

#[test]
fn compute_sector_rotation_no_data() {
    let snap = compute_sector_rotation_snapshot("X", "2026-04-14", "Technology", &[]);
    assert_eq!(snap.strength_label, "NO_DATA");
}

fn rc(date: &str, action: &str, firm: &str, to: &str) -> RatingChange {
    RatingChange {
        date: date.to_string(),
        symbol: "X".into(),
        company: String::new(),
        firm: firm.to_string(),
        action: action.to_string(),
        from_grade: String::new(),
        to_grade: to.to_string(),
        ..Default::default()
    }
}

#[test]
fn updm_snapshot_roundtrip() {
    let c = open_mem_conn_v12();
    let snap = UpdmSnapshot {
        symbol: "NVDA".into(),
        as_of: "2026-04-14".into(),
        total_actions: 8,
        upgrades_90d: 5,
        downgrades_90d: 2,
        net_90d: 3,
        latest_date: "2026-04-10".into(),
        latest_action: "upgrade".into(),
        bias_label: "BULLISH".into(),
        trend_label: "IMPROVING".into(),
        ..Default::default()
    };
    upsert_updm(&c, "NVDA", &snap).unwrap();
    let got = get_updm(&c, "nvda").unwrap().unwrap();
    assert_eq!(got.bias_label, "BULLISH");
    assert_eq!(got.net_90d, 3);
}

#[test]
fn compute_updm_bullish_improving() {
    let actions = vec![
        rc("2026-04-10", "upgrade", "Morgan Stanley", "Overweight"),
        rc("2026-04-05", "upgrade", "Goldman Sachs", "Buy"),
        rc("2026-03-28", "initiation", "JPM", "Overweight"),
        rc("2026-03-15", "downgrade", "Bernstein", "Market-Perform"),
        rc("2026-02-20", "upgrade", "Wells Fargo", "Outperform"),
        rc("2025-12-10", "downgrade", "Citi", "Neutral"),
    ];
    let snap = compute_updm_snapshot("NVDA", "2026-04-14", &actions);
    assert_eq!(snap.bias_label, "BULLISH");
    assert!(snap.net_90d > 0);
    assert_eq!(snap.latest_date, "2026-04-10");
    assert_eq!(snap.latest_action, "upgrade");
}

#[test]
fn compute_updm_bearish() {
    let actions = vec![
        rc("2026-04-10", "downgrade", "Morgan Stanley", "Underweight"),
        rc("2026-04-01", "downgrade", "Goldman Sachs", "Sell"),
        rc("2026-03-20", "downgrade", "Bernstein", "Market-Perform"),
        rc("2026-03-10", "upgrade", "Wells Fargo", "Outperform"),
    ];
    let snap = compute_updm_snapshot("X", "2026-04-14", &actions);
    assert_eq!(snap.bias_label, "BEARISH");
    assert!(snap.net_90d < 0);
}

#[test]
fn compute_updm_no_coverage() {
    let snap = compute_updm_snapshot("X", "2026-04-14", &[]);
    assert_eq!(snap.bias_label, "NO_COVERAGE");
}

// Market liquidity, momentum, breakout, cash-cycle, and credit tests

fn open_mem_conn_v13() -> Connection {
    let c = Connection::open_in_memory().unwrap();
    create_research_tables_v13(&c).unwrap();
    c
}

fn make_bar(date: &str, close: f64, volume: f64) -> HistoricalPriceRow {
    HistoricalPriceRow {
        date: date.to_string(),
        open: close,
        high: close * 1.01,
        low: close * 0.99,
        close,
        adj_close: close,
        volume,
        change: 0.0,
        change_pct: 0.0,
    }
}

fn make_bar_flat(date: &str, close: f64, volume: f64) -> HistoricalPriceRow {
    HistoricalPriceRow {
        date: date.to_string(),
        open: close,
        high: close,
        low: close,
        close,
        adj_close: close,
        volume,
        change: 0.0,
        change_pct: 0.0,
    }
}

#[test]
fn momentum_snapshot_roundtrip() {
    let c = open_mem_conn_v13();
    let snap = MomentumSnapshot {
        symbol: "X".to_string(),
        as_of: "2026-04-14".to_string(),
        bars_used: 252,
        return_12m_pct: 35.0,
        regime_label: "STRONG".to_string(),
        trend_label: "ACCELERATING".to_string(),
        ..Default::default()
    };
    upsert_momentum(&c, "x", &snap).unwrap();
    let got = get_momentum(&c, "X").unwrap().unwrap();
    assert_eq!(got.symbol, "X");
    assert_eq!(got.regime_label, "STRONG");
}

#[test]
fn liquidity_snapshot_roundtrip() {
    let c = open_mem_conn_v13();
    let snap = LiquiditySnapshot {
        symbol: "Y".to_string(),
        as_of: "2026-04-14".to_string(),
        window_days: 60,
        avg_daily_dollar_volume: 1.5e9,
        liquidity_tier: "DEEP".to_string(),
        ..Default::default()
    };
    upsert_liquidity(&c, "y", &snap).unwrap();
    let got = get_liquidity(&c, "Y").unwrap().unwrap();
    assert_eq!(got.liquidity_tier, "DEEP");
}

#[test]
fn breakout_snapshot_roundtrip() {
    let c = open_mem_conn_v13();
    let snap = BreakoutSnapshot {
        symbol: "Z".to_string(),
        as_of: "2026-04-14".to_string(),
        current_price: 100.0,
        high_52w: 100.0,
        low_52w: 60.0,
        position_in_52w_range_pct: 100.0,
        breakout_label: "NEW_HIGH".to_string(),
        setup_label: "TRENDING_UP".to_string(),
        ..Default::default()
    };
    upsert_breakout(&c, "z", &snap).unwrap();
    let got = get_breakout(&c, "Z").unwrap().unwrap();
    assert_eq!(got.breakout_label, "NEW_HIGH");
}

#[test]
fn cash_cycle_snapshot_roundtrip() {
    let c = open_mem_conn_v13();
    let snap = CashCycleSnapshot {
        symbol: "W".to_string(),
        as_of: "2026-04-14".to_string(),
        latest_period: "2025-12-31".to_string(),
        ccc_days: 45.0,
        efficiency_label: "NEUTRAL".to_string(),
        trend_label: "STABLE".to_string(),
        periods: vec![CashCycleRow {
            period: "2025-12-31".to_string(),
            dso_days: 40.0,
            dio_days: 30.0,
            dpo_days: 25.0,
            ccc_days: 45.0,
        }],
        ..Default::default()
    };
    upsert_cash_cycle(&c, "w", &snap).unwrap();
    let got = get_cash_cycle(&c, "W").unwrap().unwrap();
    assert_eq!(got.latest_period, "2025-12-31");
    assert_eq!(got.periods.len(), 1);
}

#[test]
fn credit_snapshot_roundtrip() {
    let c = open_mem_conn_v13();
    let snap = CreditSnapshot {
        symbol: "V".to_string(),
        as_of: "2026-04-14".to_string(),
        composite_score: 78.0,
        letter_grade: "A".to_string(),
        credit_label: "INVESTMENT_GRADE".to_string(),
        inputs_available: 4,
        ..Default::default()
    };
    upsert_credit(&c, "v", &snap).unwrap();
    let got = get_credit(&c, "V").unwrap().unwrap();
    assert_eq!(got.letter_grade, "A");
    assert_eq!(got.credit_label, "INVESTMENT_GRADE");
}

#[test]
fn compute_momentum_strong() {
    // 260 bars, steadily rising from 100 → 140 with low vol.
    // Needs > 252 bars because compute_momentum_snapshot reads offset 252.
    let mut bars: Vec<HistoricalPriceRow> = Vec::new();
    for i in 0..260 {
        // newest-first: i=0 is the newest bar
        let days_old = i as f64;
        let close = 140.0 - days_old * (40.0 / 260.0); // newest=140, oldest≈100
        bars.push(make_bar(
            &format!("2026-{:02}-{:02}", 1 + (i / 30) % 12, 1 + (i % 28)),
            close,
            1_000_000.0,
        ));
    }
    let snap = compute_momentum_snapshot("AAA", "2026-04-14", &bars);
    assert!(snap.bars_used >= 252);
    assert!(snap.return_12m_pct > 0.0);
    assert_ne!(snap.regime_label, "INSUFFICIENT_DATA");
}

#[test]
fn compute_momentum_insufficient() {
    let bars: Vec<HistoricalPriceRow> = (0..50)
        .map(|i| make_bar(&format!("2026-04-{:02}", (i % 28) + 1), 100.0, 1_000.0))
        .collect();
    let snap = compute_momentum_snapshot("AAA", "2026-04-14", &bars);
    assert_eq!(snap.regime_label, "INSUFFICIENT_DATA");
}

#[test]
fn compute_liquidity_deep() {
    // 60 bars, large dollar volume
    let bars: Vec<HistoricalPriceRow> = (0..60)
        .map(|i| make_bar(&format!("2026-04-{:02}", (i % 28) + 1), 200.0, 10_000_000.0))
        .collect();
    let snap = compute_liquidity_snapshot("BBB", "2026-04-14", &bars, 1_000_000_000.0, 60);
    assert_eq!(snap.liquidity_tier, "DEEP");
    assert!(snap.avg_daily_dollar_volume > 1.0e9);
}

#[test]
fn compute_liquidity_thin() {
    let bars: Vec<HistoricalPriceRow> = (0..60)
        .map(|i| make_bar(&format!("2026-04-{:02}", (i % 28) + 1), 5.0, 1_000.0))
        .collect();
    let snap = compute_liquidity_snapshot("BBB", "2026-04-14", &bars, 100_000_000.0, 60);
    assert!(matches!(snap.liquidity_tier.as_str(), "THIN" | "ILLIQUID"));
}

#[test]
fn compute_liquidity_insufficient() {
    let bars: Vec<HistoricalPriceRow> = (0..10)
        .map(|i| make_bar(&format!("2026-04-{:02}", i + 1), 100.0, 1_000.0))
        .collect();
    let snap = compute_liquidity_snapshot("BBB", "2026-04-14", &bars, 1_000_000.0, 60);
    assert_eq!(snap.liquidity_tier, "INSUFFICIENT_DATA");
}

#[test]
fn compute_breakout_new_high() {
    // Uses flat bars (high = low = close) so `current >= h52` holds.
    let mut bars: Vec<HistoricalPriceRow> = Vec::new();
    for i in 0..252 {
        let close = 100.0 - (i as f64) * 0.3; // newest = 100, oldest = ~24
        bars.push(make_bar_flat(
            &format!("2025-{:02}-{:02}", (1 + i / 30) % 12 + 1, (i % 28) + 1),
            close,
            1_000_000.0,
        ));
    }
    let snap = compute_breakout_snapshot("CCC", "2026-04-14", &bars);
    assert_eq!(snap.breakout_label, "NEW_HIGH");
    assert!(snap.position_in_52w_range_pct >= 99.0);
}

#[test]
fn compute_breakout_near_low() {
    // Newest bar is near the 52w low (older bars are higher)
    let mut bars: Vec<HistoricalPriceRow> = Vec::new();
    for i in 0..252 {
        let close = 10.0 + (i as f64) * 0.3;
        bars.push(make_bar_flat(
            &format!("2025-{:02}-{:02}", (1 + i / 30) % 12 + 1, (i % 28) + 1),
            close,
            1_000_000.0,
        ));
    }
    let snap = compute_breakout_snapshot("CCC", "2026-04-14", &bars);
    assert!(matches!(
        snap.breakout_label.as_str(),
        "NEAR_LOW" | "NEW_LOW"
    ));
}

#[test]
fn compute_breakout_insufficient() {
    let bars: Vec<HistoricalPriceRow> = (0..5)
        .map(|i| make_bar(&format!("2026-04-{:02}", i + 1), 100.0, 1_000.0))
        .collect();
    let snap = compute_breakout_snapshot("CCC", "2026-04-14", &bars);
    assert_eq!(snap.breakout_label, "INSUFFICIENT_DATA");
}

#[test]
fn compute_cash_cycle_efficient() {
    let income = IncomeStatement {
        date: "2025-12-31".to_string(),
        period: "FY".to_string(),
        revenue: 10_000.0,
        cost_of_revenue: 6_000.0,
        ..Default::default()
    };
    let income_prior = IncomeStatement {
        date: "2024-12-31".to_string(),
        period: "FY".to_string(),
        revenue: 9_000.0,
        cost_of_revenue: 5_400.0,
        ..Default::default()
    };
    let balance = BalanceSheet {
        date: "2025-12-31".to_string(),
        period: "FY".to_string(),
        net_receivables: 400.0,  // ~14.6 DSO
        inventory: 300.0,        // ~18.25 DIO
        accounts_payable: 900.0, // ~54.75 DPO → CCC ≈ -21.9
        ..Default::default()
    };
    let balance_prior = BalanceSheet {
        date: "2024-12-31".to_string(),
        period: "FY".to_string(),
        net_receivables: 500.0,
        inventory: 350.0,
        accounts_payable: 850.0,
        ..Default::default()
    };
    let statements = FinancialStatements {
        income_annual: vec![income, income_prior],
        balance_annual: vec![balance, balance_prior],
        ..Default::default()
    };
    let snap = compute_cash_cycle_snapshot("DDD", "2026-04-14", &statements);
    assert!(snap.ccc_days < 30.0);
    assert_eq!(snap.efficiency_label, "EFFICIENT");
    assert_eq!(snap.periods.len(), 2);
}

#[test]
fn compute_cash_cycle_insufficient() {
    let statements = FinancialStatements::default();
    let snap = compute_cash_cycle_snapshot("DDD", "2026-04-14", &statements);
    assert_eq!(snap.efficiency_label, "INSUFFICIENT_DATA");
}

#[test]
fn compute_credit_investment_grade() {
    let altz = AltmanZSnapshot {
        z_score: 4.5,
        zone: "SAFE".to_string(),
        ..Default::default()
    };
    let ptfs = PiotroskiSnapshot {
        f_score: 8,
        strength_label: "STRONG".to_string(),
        ..Default::default()
    };
    let lev = LeverageSnapshot {
        solvency_summary: "HEALTHY".to_string(),
        ..Default::default()
    };
    let acrl = AccrualsSnapshot {
        trend_label: "IMPROVING".to_string(),
        ttm_cash_conversion_pct: 120.0,
        ..Default::default()
    };
    let snap = compute_credit_snapshot(
        "EEE",
        "2026-04-14",
        Some(&altz),
        Some(&ptfs),
        Some(&lev),
        Some(&acrl),
    );
    assert_eq!(snap.credit_label, "INVESTMENT_GRADE");
    assert!(snap.composite_score >= 70.0);
    assert_eq!(snap.inputs_available, 4);
    assert_eq!(snap.components.len(), 4);
}

#[test]
fn compute_credit_distressed() {
    let altz = AltmanZSnapshot {
        z_score: 0.8,
        zone: "DISTRESS".to_string(),
        ..Default::default()
    };
    let ptfs = PiotroskiSnapshot {
        f_score: 1,
        strength_label: "WEAK".to_string(),
        ..Default::default()
    };
    let lev = LeverageSnapshot {
        solvency_summary: "STRETCHED".to_string(),
        ..Default::default()
    };
    let acrl = AccrualsSnapshot {
        trend_label: "DETERIORATING".to_string(),
        ttm_cash_conversion_pct: 30.0,
        ..Default::default()
    };
    let snap = compute_credit_snapshot(
        "EEE",
        "2026-04-14",
        Some(&altz),
        Some(&ptfs),
        Some(&lev),
        Some(&acrl),
    );
    assert_eq!(snap.credit_label, "DISTRESSED");
    assert!(snap.composite_score < 35.0);
}

#[test]
fn compute_credit_no_inputs() {
    let snap = compute_credit_snapshot("EEE", "2026-04-14", None, None, None, None);
    assert_eq!(snap.letter_grade, "INSUFFICIENT_DATA");
    assert_eq!(snap.inputs_available, 0);
}

// ── Research section ──

#[test]
fn growm_snapshot_roundtrip() {
    let c = Connection::open_in_memory().unwrap();
    create_research_tables_v14(&c).unwrap();
    let snap = GrowmSnapshot {
        symbol: "AAA".to_string(),
        as_of: "2026-04-14".to_string(),
        composite_score: 82.5,
        garp_label: "GARP".to_string(),
        inputs_available: 3,
        ..Default::default()
    };
    upsert_growm(&c, "aaa", &snap).unwrap();
    let got = get_growm(&c, "AAA").unwrap().unwrap();
    assert_eq!(got.garp_label, "GARP");
    assert_eq!(got.inputs_available, 3);
}

#[test]
fn flow_snapshot_roundtrip() {
    let c = Connection::open_in_memory().unwrap();
    create_research_tables_v14(&c).unwrap();
    let snap = FlowSnapshot {
        symbol: "BBB".to_string(),
        as_of: "2026-04-14".to_string(),
        window_days: 90,
        composite_score: 72.0,
        flow_label: "BUY".to_string(),
        ..Default::default()
    };
    upsert_flow(&c, "bbb", &snap).unwrap();
    let got = get_flow(&c, "BBB").unwrap().unwrap();
    assert_eq!(got.flow_label, "BUY");
}

#[test]
fn regime_snapshot_roundtrip() {
    let c = Connection::open_in_memory().unwrap();
    create_research_tables_v14(&c).unwrap();
    let snap = RegimeSnapshot {
        symbol: "CCC".to_string(),
        as_of: "2026-04-14".to_string(),
        regime_label: "TRENDING".to_string(),
        inputs_available: 3,
        ..Default::default()
    };
    upsert_regime(&c, "ccc", &snap).unwrap();
    let got = get_regime(&c, "CCC").unwrap().unwrap();
    assert_eq!(got.regime_label, "TRENDING");
}

#[test]
fn relvol_snapshot_roundtrip() {
    let c = Connection::open_in_memory().unwrap();
    create_research_tables_v14(&c).unwrap();
    let snap = RelVolSnapshot {
        symbol: "DDD".to_string(),
        as_of: "2026-04-14".to_string(),
        activity_label: "HIGH".to_string(),
        direction_label: "BULLISH".to_string(),
        bars_used: 60,
        ..Default::default()
    };
    upsert_relvol(&c, "ddd", &snap).unwrap();
    let got = get_relvol(&c, "DDD").unwrap().unwrap();
    assert_eq!(got.activity_label, "HIGH");
}

#[test]
fn margins_snapshot_roundtrip() {
    let c = Connection::open_in_memory().unwrap();
    create_research_tables_v14(&c).unwrap();
    let snap = MarginsSnapshot {
        symbol: "EEE".to_string(),
        as_of: "2026-04-14".to_string(),
        basis: "annual".to_string(),
        overall_trend_label: "EXPANDING".to_string(),
        quality_label: "HIGH".to_string(),
        ..Default::default()
    };
    upsert_margins(&c, "eee", &snap).unwrap();
    let got = get_margins(&c, "EEE").unwrap().unwrap();
    assert_eq!(got.overall_trend_label, "EXPANDING");
}

#[test]
fn compute_growm_garp() {
    let mom = MomentumSnapshot {
        composite_score: 78.0,
        regime_label: "STRONG".to_string(),
        ..Default::default()
    };
    let earm = EarmSnapshot {
        composite_score: 72.0,
        momentum_label: "ACCELERATING".to_string(),
        ..Default::default()
    };
    let divg = DivgSnapshot {
        cagr_3y_pct: 8.0,
        trend_label: "GROWING".to_string(),
        ..Default::default()
    };
    let snap = compute_growm_snapshot("AAA", "2026-04-14", Some(&mom), Some(&earm), Some(&divg));
    assert!(snap.composite_score >= 65.0);
    assert_eq!(snap.inputs_available, 3);
    assert!(matches!(snap.garp_label.as_str(), "GARP" | "GROWTH"));
}

#[test]
fn compute_growm_no_inputs() {
    let snap = compute_growm_snapshot("AAA", "2026-04-14", None, None, None);
    assert_eq!(snap.garp_label, "NO_DATA");
    assert_eq!(snap.inputs_available, 0);
}

#[test]
fn compute_flow_buy() {
    let trades = vec![
        InsiderTrade {
            transaction_date: "2026-04-10".to_string(),
            reporting_name: "Alice CFO".to_string(),
            transaction_type: "P-Purchase".to_string(),
            value_usd: 500_000.0,
            ..Default::default()
        },
        InsiderTrade {
            transaction_date: "2026-04-01".to_string(),
            reporting_name: "Bob CEO".to_string(),
            transaction_type: "P-Purchase".to_string(),
            value_usd: 800_000.0,
            ..Default::default()
        },
    ];
    let holders = vec![
        InstitutionalHolder {
            holder: "X Fund".to_string(),
            change: 100_000.0,
            ..Default::default()
        },
        InstitutionalHolder {
            holder: "Y Fund".to_string(),
            change: 50_000.0,
            ..Default::default()
        },
        InstitutionalHolder {
            holder: "Z Fund".to_string(),
            change: -20_000.0,
            ..Default::default()
        },
    ];
    let snap = compute_flow_snapshot("BBB", "2026-04-14", &trades, &holders, 90);
    assert!(matches!(snap.flow_label.as_str(), "BUY" | "STRONG_BUY"));
    assert_eq!(snap.insider_trade_count, 2);
    assert_eq!(snap.institutional_buyers, 2);
    assert_eq!(snap.institutional_sellers, 1);
}

#[test]
fn compute_flow_no_data() {
    let snap = compute_flow_snapshot("BBB", "2026-04-14", &[], &[], 90);
    assert_eq!(snap.flow_label, "NO_DATA");
}

#[test]
fn compute_regime_trending() {
    let tech = TechnicalSnapshot {
        indicators: vec![TechnicalIndicator {
            name: "ADX(14)".to_string(),
            value: 32.0,
            ..Default::default()
        }],
        trend_summary: "bullish trend".to_string(),
        ..Default::default()
    };
    let vole = OhlcVolSnapshot {
        preferred_estimate_pct: 18.0,
        preferred_label: "Yang-Zhang".to_string(),
        ..Default::default()
    };
    let hra = HraSnapshot {
        sharpe_ratio: 1.8,
        volatility_annual_pct: 18.0,
        windows: vec![HraWindow {
            label: "1Y".to_string(),
            return_pct: 22.0,
            ..Default::default()
        }],
        ..Default::default()
    };
    let snap = compute_regime_snapshot("CCC", "2026-04-14", Some(&vole), Some(&tech), Some(&hra));
    assert_eq!(snap.regime_label, "TRENDING");
    assert_eq!(snap.inputs_available, 3);
}

#[test]
fn compute_regime_volatile() {
    let vole = OhlcVolSnapshot {
        preferred_estimate_pct: 55.0,
        preferred_label: "Yang-Zhang".to_string(),
        ..Default::default()
    };
    let snap = compute_regime_snapshot("CCC", "2026-04-14", Some(&vole), None, None);
    assert_eq!(snap.regime_label, "VOLATILE");
}

#[test]
fn compute_regime_no_inputs() {
    let snap = compute_regime_snapshot("CCC", "2026-04-14", None, None, None);
    assert_eq!(snap.regime_label, "INSUFFICIENT_DATA");
}

#[test]
fn compute_relvol_high() {
    let mut bars: Vec<HistoricalPriceRow> = Vec::new();
    // Current bar (index 0) has 5x avg volume.
    bars.push(HistoricalPriceRow {
        date: "2026-04-14".to_string(),
        volume: 5_000_000.0,
        close: 105.0,
        ..Default::default()
    });
    for i in 1..=60 {
        bars.push(HistoricalPriceRow {
            date: format!("2026-04-{:02}", 14 - (i % 14)),
            volume: 1_000_000.0,
            close: 100.0,
            ..Default::default()
        });
    }
    let snap = compute_relvol_snapshot("DDD", "2026-04-14", &bars);
    assert!(snap.rel_volume_20d >= 4.0);
    assert_eq!(snap.activity_label, "EXTREME");
    assert_eq!(snap.direction_label, "BULLISH");
}

#[test]
fn compute_relvol_insufficient() {
    let bars: Vec<HistoricalPriceRow> = (0..10)
        .map(|i| HistoricalPriceRow {
            date: format!("2026-04-{:02}", 14 - i),
            volume: 1_000.0,
            close: 100.0,
            ..Default::default()
        })
        .collect();
    let snap = compute_relvol_snapshot("DDD", "2026-04-14", &bars);
    assert_eq!(snap.activity_label, "INSUFFICIENT_DATA");
}

#[test]
fn compute_margins_expanding() {
    let latest = IncomeStatement {
        date: "2025-12-31".to_string(),
        period: "FY".to_string(),
        revenue: 10_000.0,
        gross_profit: 4_500.0,     // 45%
        operating_income: 2_500.0, // 25%
        net_income: 1_800.0,       // 18%
        ..Default::default()
    };
    let prior = IncomeStatement {
        date: "2024-12-31".to_string(),
        period: "FY".to_string(),
        revenue: 9_000.0,
        gross_profit: 3_600.0,     // 40%
        operating_income: 1_800.0, // 20%
        net_income: 1_350.0,       // 15%
        ..Default::default()
    };
    let statements = FinancialStatements {
        income_annual: vec![latest, prior],
        ..Default::default()
    };
    let snap = compute_margins_snapshot("EEE", "2026-04-14", &statements);
    assert_eq!(snap.overall_trend_label, "EXPANDING");
    assert_eq!(snap.quality_label, "HIGH");
    assert_eq!(snap.periods_used, 2);
    assert!(snap.operating_margin_change_pct > 0.0);
}

#[test]
fn compute_margins_insufficient() {
    let statements = FinancialStatements::default();
    let snap = compute_margins_snapshot("EEE", "2026-04-14", &statements);
    assert_eq!(snap.overall_trend_label, "INSUFFICIENT_DATA");
}
