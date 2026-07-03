#[test]
fn commodities_universe_has_expected_sectors() {
    let sectors: std::collections::HashSet<&str> =
        COMMODITIES_UNIVERSE.iter().map(|(_, _, s)| *s).collect();
    assert!(sectors.contains("Metals"));
    assert!(sectors.contains("Energy"));
    assert!(sectors.contains("Grains"));
    assert!(sectors.contains("Softs"));
    assert!(sectors.contains("Livestock"));
}

#[test]
fn commodities_universe_all_yahoo_futures_format() {
    for (sym, _, _) in COMMODITIES_UNIVERSE {
        assert!(sym.ends_with("=F"), "{} should end with =F", sym);
    }
}

#[test]
fn company_profile_default_is_empty() {
    let p = CompanyProfile::default();
    assert!(p.symbol.is_empty());
    assert_eq!(p.market_cap, 0.0);
}

#[test]
fn earning_row_all_optional() {
    let r = EarningRow::default();
    assert!(r.actual.is_none());
    assert!(r.estimate.is_none());
    assert!(r.surprise.is_none());
}

#[test]
fn transcript_meta_roundtrip_json() {
    let m = TranscriptMeta {
        symbol: "AAPL".into(),
        quarter: 4,
        year: 2023,
        date: "2024-02-01".into(),
    };
    let j = serde_json::to_string(&m).unwrap();
    let b: TranscriptMeta = serde_json::from_str(&j).unwrap();
    assert_eq!(b.symbol, "AAPL");
    assert_eq!(b.quarter, 4);
}

#[test]
fn transcript_summary_extracts_prepared_and_qa_sections() {
    let t = Transcript {
            symbol: "AAPL".into(),
            quarter: 1,
            year: 2026,
            date: "2026-01-30".into(),
            content: "Operator\nWelcome to the call.\n\nTim Cook\nRevenue grew across services and emerging markets with strong gross margin performance and a record installed base. Management highlighted continued investment discipline and customer demand across the portfolio.\n\nLuca Maestri\nOperating cash flow remained strong, capital returns continued, and the company ended the quarter with a balanced capital structure.\n\nQuestion-and-Answer Session\n\nAnalyst\nCan you discuss services growth?\n\nTim Cook\nServices demand remained broad based, paid subscriptions increased, and enterprise adoption continued to support recurring revenue visibility into the next quarter.".into(),
        };

    let summary = summarize_transcript(&t);
    assert!(summary.headline.contains("AAPL Q1 2026"));
    assert!(summary.bullets.iter().any(|b| b.contains("Revenue grew")));
    assert!(
        summary
            .sections
            .iter()
            .any(|s| s.title == "Prepared remarks")
    );
    assert!(summary.sections.iter().any(|s| s.title == "Q&A"));
}

// ── ─────────────────────────────────────────────────────────

fn open_mem_conn() -> Connection {
    let c = Connection::open_in_memory().unwrap();
    create_research_tables_v2(&c).unwrap();
    c
}

#[test]
fn social_snapshots_roundtrip_and_feed_history() {
    let c = open_mem_conn();
    let st = StockTwitsSentimentSnapshot {
        symbol: "WOK".into(),
        fetched_at: "2026-07-03T12:00:00Z".into(),
        bullish: 7,
        bearish: 3,
        neutral: 2,
        message_count: 12,
        bull_bear_ratio: 7.0 / 3.0,
        velocity_24h: 12,
        top_messages: Vec::new(),
    };
    upsert_stocktwits_sentiment(&c, "wok", &st).unwrap();
    let rd = RedditMentionSnapshot {
        symbol: "WOK".into(),
        fetched_at: "2026-07-03T12:00:05Z".into(),
        mentions_24h: 9,
        score_sum_24h: 431,
        comments_sum_24h: 88,
        top_posts: vec![RedditPost {
            title: "WOK squeeze".into(),
            subreddit: "wallstreetbets".into(),
            score: 400,
            num_comments: 80,
            created_utc: 1_780_000_000,
            permalink: "/r/wallstreetbets/x".into(),
        }],
    };
    upsert_reddit_mentions(&c, "wok", &rd).unwrap();

    let got_rd = get_reddit_mentions(&c, "WOK").unwrap().unwrap();
    assert_eq!(got_rd.mentions_24h, 9);
    assert_eq!(got_rd.top_posts.len(), 1);

    // Both upserts appended history points (per-second PK: same-second
    // duplicates replace, so >=1 per source is the guarantee here).
    let history = get_social_history(&c, "WOK", 50).unwrap();
    assert!(history.iter().any(|p| p.source == "stocktwits" && p.bullish == 7));
    assert!(history.iter().any(|p| p.source == "reddit" && p.messages == 9));
}

#[test]
fn dividend_record_roundtrip() {
    let c = open_mem_conn();
    let rows = vec![DividendRecord {
        ex_date: "2024-11-01".into(),
        pay_date: "2024-11-14".into(),
        record_date: "2024-11-04".into(),
        declaration_date: "2024-10-15".into(),
        amount: 0.24,
        adjusted_amount: 0.24,
        label: "Regular Cash".into(),
    }];
    upsert_dividends(&c, "AAPL", &rows).unwrap();
    let got = get_dividends(&c, "aapl").unwrap().unwrap();
    assert_eq!(got.len(), 1);
    assert_eq!(got[0].amount, 0.24);
    assert_eq!(got[0].label, "Regular Cash");
}

#[test]
fn earnings_estimate_roundtrip() {
    let c = open_mem_conn();
    let rows = vec![EarningsEstimate {
        date: "2025-12-31".into(),
        eps_avg: 2.45,
        eps_high: 2.60,
        eps_low: 2.30,
        revenue_avg: 123_000_000.0,
        revenue_high: 128_000_000.0,
        revenue_low: 118_000_000.0,
        num_analysts_eps: 12,
        num_analysts_rev: 12,
    }];
    upsert_earnings_estimates(&c, "MSFT", &rows).unwrap();
    let got = get_earnings_estimates(&c, "MSFT").unwrap().unwrap();
    assert_eq!(got.len(), 1);
    assert!((got[0].eps_avg - 2.45).abs() < 1e-9);
    assert_eq!(got[0].num_analysts_eps, 12);
}

#[test]
fn rating_change_roundtrip() {
    let c = open_mem_conn();
    let rows = vec![RatingChange {
        date: "2024-03-01".into(),
        symbol: "AAPL".into(),
        company: "Apple Inc.".into(),
        firm: "Morgan Stanley".into(),
        action: "upgrade".into(),
        from_grade: "Hold".into(),
        to_grade: "Buy".into(),
        price_target: 220.0,
    }];
    upsert_rating_changes(&c, "AAPL", &rows).unwrap();
    let got = get_rating_changes(&c, "AAPL").unwrap().unwrap();
    assert_eq!(got.len(), 1);
    assert_eq!(got[0].action, "upgrade");
    assert!((got[0].price_target - 220.0).abs() < 1e-9);
}

#[test]
fn treasury_tenor_ladder_has_four_rungs() {
    let tenors: std::collections::HashSet<&str> = TREASURY_TENORS.iter().map(|(_, t)| *t).collect();
    assert!(tenors.contains("13W"));
    assert!(tenors.contains("5Y"));
    assert!(tenors.contains("10Y"));
    assert!(tenors.contains("30Y"));
}

#[test]
fn treasury_yield_default_is_empty() {
    let y = TreasuryYield::default();
    assert!(y.tenor.is_empty());
    assert_eq!(y.yield_pct, 0.0);
}

#[test]
fn dividend_upsert_overwrites() {
    let c = open_mem_conn();
    upsert_dividends(
        &c,
        "IBM",
        &[DividendRecord {
            ex_date: "2024-05-01".into(),
            amount: 1.66,
            ..Default::default()
        }],
    )
    .unwrap();
    upsert_dividends(
        &c,
        "IBM",
        &[
            DividendRecord {
                ex_date: "2024-05-01".into(),
                amount: 1.67,
                ..Default::default()
            },
            DividendRecord {
                ex_date: "2024-08-01".into(),
                amount: 1.67,
                ..Default::default()
            },
        ],
    )
    .unwrap();
    let rows = get_dividends(&c, "IBM").unwrap().unwrap();
    assert_eq!(rows.len(), 2);
}

// ── ─────────────────────────────────────────────────────────

fn open_mem_conn_v3() -> Connection {
    let c = Connection::open_in_memory().unwrap();
    create_research_tables_v3(&c).unwrap();
    c
}

#[test]
fn financials_bundle_default_is_empty() {
    let b = FinancialStatements::default();
    assert!(b.income_annual.is_empty());
    assert!(b.income_quarterly.is_empty());
    assert!(b.balance_annual.is_empty());
    assert!(b.balance_quarterly.is_empty());
    assert!(b.cashflow_annual.is_empty());
    assert!(b.cashflow_quarterly.is_empty());
}

#[test]
fn financials_bundle_roundtrip() {
    let c = open_mem_conn_v3();
    let mut b = FinancialStatements::default();
    b.income_annual.push(IncomeStatement {
        date: "2024-09-30".into(),
        period: "FY".into(),
        revenue: 400_000_000_000.0,
        net_income: 97_000_000_000.0,
        ebitda: 135_000_000_000.0,
        eps: 6.12,
        eps_diluted: 6.08,
        ..Default::default()
    });
    b.balance_quarterly.push(BalanceSheet {
        date: "2024-06-30".into(),
        period: "Q3".into(),
        total_assets: 350_000_000_000.0,
        total_liabilities: 270_000_000_000.0,
        total_equity: 80_000_000_000.0,
        total_debt: 110_000_000_000.0,
        ..Default::default()
    });
    b.cashflow_annual.push(CashFlowStatement {
        date: "2024-09-30".into(),
        period: "FY".into(),
        cash_from_operations: 118_000_000_000.0,
        capex: -11_000_000_000.0,
        free_cash_flow: 107_000_000_000.0,
        ..Default::default()
    });
    upsert_financials(&c, "AAPL", &b).unwrap();
    let got = get_financials(&c, "aapl").unwrap().unwrap();
    assert_eq!(got.income_annual.len(), 1);
    assert_eq!(got.balance_quarterly.len(), 1);
    assert_eq!(got.cashflow_annual.len(), 1);
    assert!((got.income_annual[0].eps - 6.12).abs() < 1e-9);
    assert!((got.cashflow_annual[0].free_cash_flow - 107_000_000_000.0).abs() < 1.0);
}

#[test]
fn financials_upsert_replaces() {
    let c = open_mem_conn_v3();
    let mut b1 = FinancialStatements::default();
    b1.income_annual.push(IncomeStatement {
        date: "2023-09-30".into(),
        revenue: 1.0,
        ..Default::default()
    });
    upsert_financials(&c, "T", &b1).unwrap();
    let mut b2 = FinancialStatements::default();
    b2.income_annual.push(IncomeStatement {
        date: "2024-09-30".into(),
        revenue: 2.0,
        ..Default::default()
    });
    b2.income_annual.push(IncomeStatement {
        date: "2023-09-30".into(),
        revenue: 1.0,
        ..Default::default()
    });
    upsert_financials(&c, "T", &b2).unwrap();
    let got = get_financials(&c, "T").unwrap().unwrap();
    assert_eq!(got.income_annual.len(), 2);
}

#[test]
fn executive_roundtrip() {
    let c = open_mem_conn_v3();
    let rows = vec![
        Executive {
            name: "Tim Cook".into(),
            position: "CEO".into(),
            age: 64,
            sex: "M".into(),
            since: "2011".into(),
            compensation: 74_600_000.0,
            year: 2023,
        },
        Executive {
            name: "Luca Maestri".into(),
            position: "CFO".into(),
            age: 60,
            sex: "M".into(),
            since: "2014".into(),
            compensation: 27_100_000.0,
            year: 2023,
        },
    ];
    upsert_executives(&c, "AAPL", &rows).unwrap();
    let got = get_executives(&c, "aapl").unwrap().unwrap();
    assert_eq!(got.len(), 2);
    assert_eq!(got[0].name, "Tim Cook");
    assert!((got[1].compensation - 27_100_000.0).abs() < 1.0);
}

#[test]
fn cot_report_default_is_empty() {
    let r = CotReport::default();
    assert!(r.market_name.is_empty());
    assert_eq!(r.open_interest, 0.0);
    assert_eq!(r.noncomm_net, 0.0);
    assert_eq!(r.noncomm_net_change, 0.0);
}

#[test]
fn cot_report_net_math() {
    // Derived invariant used by the UI's coloring / direction signal.
    let r = CotReport {
        noncomm_long: 120_000.0,
        noncomm_short: 45_000.0,
        noncomm_net: 120_000.0 - 45_000.0,
        noncomm_net_change: 5_000.0,
        ..Default::default()
    };
    assert!((r.noncomm_net - 75_000.0).abs() < 1e-9);
    assert!(r.noncomm_net_change > 0.0);
}

// ── ─────────────────────────────────────────────────────────

fn open_mem_conn_v4() -> Connection {
    let c = Connection::open_in_memory().unwrap();
    create_research_tables_v4(&c).unwrap();
    c
}

#[test]
fn stock_split_default_is_empty() {
    let s = StockSplit::default();
    assert!(s.date.is_empty());
    assert!(s.label.is_empty());
    assert_eq!(s.numerator, 0.0);
    assert_eq!(s.denominator, 0.0);
}

#[test]
fn stock_split_roundtrip() {
    let c = open_mem_conn_v4();
    let rows = vec![
        StockSplit {
            date: "2020-08-31".into(),
            label: "4:1".into(),
            numerator: 4.0,
            denominator: 1.0,
        },
        StockSplit {
            date: "2014-06-09".into(),
            label: "7:1".into(),
            numerator: 7.0,
            denominator: 1.0,
        },
    ];
    upsert_stock_splits(&c, "AAPL", &rows).unwrap();
    let got = get_stock_splits(&c, "aapl").unwrap().unwrap();
    assert_eq!(got.len(), 2);
    assert_eq!(got[0].label, "4:1");
    assert!((got[1].numerator - 7.0).abs() < 1e-9);
}

#[test]
fn etf_holding_roundtrip() {
    let c = open_mem_conn_v4();
    let rows = vec![
        EtfHolding {
            symbol: "AAPL".into(),
            name: "Apple Inc.".into(),
            weight_pct: 7.21,
            shares: 176_000_000.0,
            market_value: 34_500_000_000.0,
            updated: "2024-06-30".into(),
        },
        EtfHolding {
            symbol: "MSFT".into(),
            name: "Microsoft Corp.".into(),
            weight_pct: 6.87,
            shares: 83_000_000.0,
            market_value: 32_900_000_000.0,
            updated: "2024-06-30".into(),
        },
    ];
    upsert_etf_holdings(&c, "SPY", &rows).unwrap();
    let got = get_etf_holdings(&c, "spy").unwrap().unwrap();
    assert_eq!(got.len(), 2);
    assert_eq!(got[0].symbol, "AAPL");
    assert!((got[1].weight_pct - 6.87).abs() < 1e-9);
}

#[test]
fn analyst_rec_roundtrip() {
    let c = open_mem_conn_v4();
    let rows = vec![
        AnalystRecommendation {
            period: "2026-04-01".into(),
            strong_buy: 15,
            buy: 12,
            hold: 8,
            sell: 1,
            strong_sell: 0,
        },
        AnalystRecommendation {
            period: "2026-03-01".into(),
            strong_buy: 14,
            buy: 13,
            hold: 9,
            sell: 1,
            strong_sell: 0,
        },
    ];
    upsert_analyst_recs(&c, "AAPL", &rows).unwrap();
    let got = get_analyst_recs(&c, "AAPL").unwrap().unwrap();
    assert_eq!(got.len(), 2);
    assert_eq!(got[0].strong_buy, 15);
    assert_eq!(got[1].hold, 9);
}

#[test]
fn price_target_default_is_empty() {
    let p = PriceTarget::default();
    assert!(p.symbol.is_empty());
    assert_eq!(p.target_mean, 0.0);
    assert_eq!(p.num_analysts, 0);
}

#[test]
fn price_target_roundtrip() {
    let c = open_mem_conn_v4();
    let pt = PriceTarget {
        symbol: "NVDA".into(),
        target_high: 220.0,
        target_low: 140.0,
        target_mean: 185.50,
        target_median: 190.0,
        last_updated: "2026-04-10".into(),
        num_analysts: 45,
    };
    upsert_price_target(&c, "NVDA", &pt).unwrap();
    let got = get_price_target(&c, "nvda").unwrap().unwrap();
    assert_eq!(got.num_analysts, 45);
    assert!((got.target_mean - 185.50).abs() < 1e-9);
}

#[test]
fn price_target_upsert_replaces() {
    let c = open_mem_conn_v4();
    upsert_price_target(
        &c,
        "T",
        &PriceTarget {
            symbol: "T".into(),
            target_mean: 20.0,
            num_analysts: 10,
            ..Default::default()
        },
    )
    .unwrap();
    upsert_price_target(
        &c,
        "T",
        &PriceTarget {
            symbol: "T".into(),
            target_mean: 22.5,
            num_analysts: 12,
            ..Default::default()
        },
    )
    .unwrap();
    let got = get_price_target(&c, "T").unwrap().unwrap();
    assert_eq!(got.num_analysts, 12);
    assert!((got.target_mean - 22.5).abs() < 1e-9);
}

#[test]
fn esg_roundtrip() {
    let c = open_mem_conn_v4();
    let rows = vec![
        EsgScore {
            symbol: "AAPL".into(),
            environmental_score: 78.5,
            social_score: 71.2,
            governance_score: 82.3,
            esg_score: 77.3,
            year: 2024,
        },
        EsgScore {
            symbol: "AAPL".into(),
            environmental_score: 76.0,
            social_score: 70.0,
            governance_score: 80.5,
            esg_score: 75.5,
            year: 2023,
        },
    ];
    upsert_esg(&c, "AAPL", &rows).unwrap();
    let got = get_esg(&c, "aapl").unwrap().unwrap();
    assert_eq!(got.len(), 2);
    assert_eq!(got[0].year, 2024);
    assert!((got[0].esg_score - 77.3).abs() < 1e-9);
}

#[test]
fn index_member_roundtrip() {
    let c = open_mem_conn_v4();
    let rows = vec![
        IndexMember {
            index: "SP500".into(),
            symbol: "AAPL".into(),
            name: "Apple Inc.".into(),
            sector: "Information Technology".into(),
            sub_sector: "Technology Hardware".into(),
            headquarters: "Cupertino, CA".into(),
            date_added: "1982-11-30".into(),
        },
        IndexMember {
            index: "SP500".into(),
            symbol: "MSFT".into(),
            name: "Microsoft Corp.".into(),
            sector: "Information Technology".into(),
            sub_sector: "Software".into(),
            headquarters: "Redmond, WA".into(),
            date_added: "1994-06-01".into(),
        },
    ];
    upsert_index_members(&c, "SP500", &rows).unwrap();
    let got = get_index_members(&c, "sp500").unwrap().unwrap();
    assert_eq!(got.len(), 2);
    assert_eq!(got[0].symbol, "AAPL");
    assert_eq!(got[1].sector, "Information Technology");
}

// ── Research section ──

fn open_mem_conn_v5() -> Connection {
    let c = Connection::open_in_memory().unwrap();
    create_research_tables_v5(&c).unwrap();
    c
}

#[test]
fn insider_trade_default_is_empty() {
    let t = InsiderTrade::default();
    assert!(t.reporting_name.is_empty());
    assert_eq!(t.shares, 0.0);
    assert_eq!(t.value_usd, 0.0);
}

#[test]
fn insider_trade_roundtrip() {
    let c = open_mem_conn_v5();
    let rows = vec![
        InsiderTrade {
            filing_date: "2026-03-10".into(),
            transaction_date: "2026-03-08".into(),
            reporting_name: "Musk, Elon".into(),
            transaction_type: "S-Sale".into(),
            acquisition_disposition: "D".into(),
            shares: 150_000.0,
            price: 245.60,
            value_usd: 150_000.0 * 245.60,
            shares_owned_after: 411_000_000.0,
            link: "https://www.sec.gov/cgi-bin/browse-edgar?action=getcompany&CIK=0001318605"
                .into(),
        },
        InsiderTrade {
            filing_date: "2026-02-11".into(),
            transaction_date: "2026-02-10".into(),
            reporting_name: "Taneja, Vaibhav".into(),
            transaction_type: "P-Purchase".into(),
            acquisition_disposition: "A".into(),
            shares: 2_500.0,
            price: 180.00,
            value_usd: 2_500.0 * 180.0,
            shares_owned_after: 42_000.0,
            link: "".into(),
        },
    ];
    upsert_insider_trades(&c, "TSLA", &rows).unwrap();
    let got = get_insider_trades(&c, "tsla").unwrap().unwrap();
    assert_eq!(got.len(), 2);
    assert_eq!(got[0].transaction_type, "S-Sale");
    assert_eq!(got[1].acquisition_disposition, "A");
    assert!((got[0].value_usd - 150_000.0 * 245.60).abs() < 1e-6);
}

#[test]
fn institutional_holder_roundtrip() {
    let c = open_mem_conn_v5();
    let rows = vec![
        InstitutionalHolder {
            holder: "Vanguard Group Inc.".into(),
            shares: 1_200_000_000.0,
            date_reported: "2025-12-31".into(),
            change: 12_000_000.0,
        },
        InstitutionalHolder {
            holder: "BlackRock Inc.".into(),
            shares: 1_050_000_000.0,
            date_reported: "2025-12-31".into(),
            change: -4_500_000.0,
        },
    ];
    upsert_institutional_holders(&c, "AAPL", &rows).unwrap();
    let got = get_institutional_holders(&c, "aapl").unwrap().unwrap();
    assert_eq!(got.len(), 2);
    assert_eq!(got[0].holder, "Vanguard Group Inc.");
    assert!(got[1].change < 0.0);
}

#[test]
fn shares_float_default_is_empty() {
    let f = SharesFloat::default();
    assert!(f.symbol.is_empty());
    assert_eq!(f.free_float_pct, 0.0);
    assert_eq!(f.outstanding_shares, 0.0);
}

#[test]
fn shares_float_roundtrip() {
    let c = open_mem_conn_v5();
    let snap = SharesFloat {
        symbol: "NVDA".into(),
        date: "2026-04-01".into(),
        free_float_pct: 95.8,
        float_shares: 23_500_000_000.0,
        outstanding_shares: 24_530_000_000.0,
        source: "FMP".into(),
    };
    upsert_shares_float(&c, "NVDA", &snap).unwrap();
    let got = get_shares_float(&c, "nvda").unwrap().unwrap();
    assert_eq!(got.symbol, "NVDA");
    assert!((got.free_float_pct - 95.8).abs() < 1e-9);
    assert!((got.outstanding_shares - 24_530_000_000.0).abs() < 1.0);
}

#[test]
fn historical_price_roundtrip() {
    let c = open_mem_conn_v5();
    let rows = vec![
        HistoricalPriceRow {
            date: "2026-04-13".into(),
            open: 180.0,
            high: 183.5,
            low: 179.2,
            close: 182.9,
            adj_close: 182.9,
            volume: 48_500_000.0,
            change: 2.9,
            change_pct: 1.61,
        },
        HistoricalPriceRow {
            date: "2026-04-12".into(),
            open: 178.1,
            high: 180.4,
            low: 177.8,
            close: 180.0,
            adj_close: 180.0,
            volume: 42_100_000.0,
            change: 1.9,
            change_pct: 1.07,
        },
    ];
    upsert_historical_price(&c, "AAPL", &rows).unwrap();
    let got = get_historical_price(&c, "aapl").unwrap().unwrap();
    assert_eq!(got.len(), 2);
    assert_eq!(got[0].date, "2026-04-13");
    assert!((got[0].change_pct - 1.61).abs() < 1e-9);
}

#[test]
fn earnings_surprise_roundtrip() {
    let c = open_mem_conn_v5();
    let rows = vec![
        EarningsSurprise {
            date: "2026-02-01".into(),
            symbol: "AAPL".into(),
            eps_actual: 2.18,
            eps_estimate: 2.11,
            surprise: 0.07,
            surprise_pct: (0.07 / 2.11) * 100.0,
        },
        EarningsSurprise {
            date: "2025-11-01".into(),
            symbol: "AAPL".into(),
            eps_actual: 1.64,
            eps_estimate: 1.60,
            surprise: 0.04,
            surprise_pct: (0.04 / 1.60) * 100.0,
        },
    ];
    upsert_earnings_surprises(&c, "AAPL", &rows).unwrap();
    let got = get_earnings_surprises(&c, "aapl").unwrap().unwrap();
    assert_eq!(got.len(), 2);
    assert!(got[0].surprise > 0.0);
    assert!((got[0].surprise_pct - (0.07 / 2.11) * 100.0).abs() < 1e-9);
}

#[test]
fn earnings_surprise_upsert_replaces() {
    let c = open_mem_conn_v5();
    upsert_earnings_surprises(
        &c,
        "T",
        &[EarningsSurprise {
            date: "2025-10-01".into(),
            symbol: "T".into(),
            eps_actual: 0.55,
            eps_estimate: 0.58,
            surprise: -0.03,
            surprise_pct: -5.17,
        }],
    )
    .unwrap();
    upsert_earnings_surprises(
        &c,
        "T",
        &[EarningsSurprise {
            date: "2026-01-01".into(),
            symbol: "T".into(),
            eps_actual: 0.60,
            eps_estimate: 0.57,
            surprise: 0.03,
            surprise_pct: 5.26,
        }],
    )
    .unwrap();
    let got = get_earnings_surprises(&c, "T").unwrap().unwrap();
    assert_eq!(got.len(), 1);
    assert_eq!(got[0].date, "2026-01-01");
    assert!(got[0].surprise > 0.0);
}

// ── Research section ──

fn open_mem_conn_v6() -> Connection {
    let c = Connection::open_in_memory().unwrap();
    create_research_tables_v6(&c).unwrap();
    c
}

#[test]
fn world_indices_universe_has_all_regions() {
    let regions: std::collections::HashSet<&str> =
        WORLD_INDICES_UNIVERSE.iter().map(|(_, _, r)| *r).collect();
    assert!(regions.contains("Americas"));
    assert!(regions.contains("EMEA"));
    assert!(regions.contains("Asia-Pacific"));
}

#[test]
fn world_indices_universe_has_sp500_and_nikkei() {
    let tickers: std::collections::HashSet<&str> =
        WORLD_INDICES_UNIVERSE.iter().map(|(t, _, _)| *t).collect();
    assert!(tickers.contains("^GSPC"));
    assert!(tickers.contains("^N225"));
    assert!(tickers.contains("^FTSE"));
}

#[test]
fn world_indices_roundtrip() {
    let c = open_mem_conn_v6();
    let rows = vec![
        WorldIndex {
            ticker: "^GSPC".into(),
            display: "S&P 500".into(),
            region: "Americas".into(),
            price: 5200.0,
            change: 12.5,
            change_pct: 0.24,
        },
        WorldIndex {
            ticker: "^N225".into(),
            display: "Nikkei 225".into(),
            region: "Asia-Pacific".into(),
            price: 39_800.0,
            change: -150.0,
            change_pct: -0.38,
        },
    ];
    upsert_world_indices(&c, &rows).unwrap();
    let got = get_world_indices(&c).unwrap().unwrap();
    assert_eq!(got.len(), 2);
    assert_eq!(got[0].ticker, "^GSPC");
    assert!(got[1].change < 0.0);
}

#[test]
fn world_indices_upsert_replaces() {
    let c = open_mem_conn_v6();
    upsert_world_indices(
        &c,
        &[WorldIndex {
            ticker: "^GSPC".into(),
            price: 5000.0,
            ..Default::default()
        }],
    )
    .unwrap();
    upsert_world_indices(
        &c,
        &[
            WorldIndex {
                ticker: "^GSPC".into(),
                price: 5300.0,
                ..Default::default()
            },
            WorldIndex {
                ticker: "^DJI".into(),
                price: 42_000.0,
                ..Default::default()
            },
        ],
    )
    .unwrap();
    let got = get_world_indices(&c).unwrap().unwrap();
    assert_eq!(got.len(), 2);
    assert!((got[0].price - 5300.0).abs() < 1e-9);
}

#[test]
fn market_movers_roundtrip() {
    let c = open_mem_conn_v6();
    let movers = MarketMovers {
        gainers: vec![MarketMover {
            symbol: "AAA".into(),
            name: "Alpha Inc.".into(),
            price: 12.5,
            change: 2.1,
            change_pct: 20.19,
            volume: 1_200_000.0,
        }],
        losers: vec![MarketMover {
            symbol: "ZZZ".into(),
            name: "Omega Corp.".into(),
            price: 4.8,
            change: -1.1,
            change_pct: -18.64,
            volume: 900_000.0,
        }],
        actives: vec![MarketMover {
            symbol: "TSLA".into(),
            name: "Tesla Inc.".into(),
            price: 190.25,
            change: 1.15,
            change_pct: 0.61,
            volume: 120_000_000.0,
        }],
    };
    upsert_market_movers(&c, &movers).unwrap();
    let got = get_market_movers(&c).unwrap().unwrap();
    assert_eq!(got.gainers.len(), 1);
    assert_eq!(got.losers.len(), 1);
    assert_eq!(got.actives.len(), 1);
    assert_eq!(got.gainers[0].symbol, "AAA");
    assert!(got.losers[0].change_pct < 0.0);
    assert_eq!(got.actives[0].symbol, "TSLA");
}

#[test]
fn sector_performance_roundtrip() {
    let c = open_mem_conn_v6();
    let rows = vec![
        SectorPerformance {
            sector: "Technology".into(),
            change_pct: 1.23,
        },
        SectorPerformance {
            sector: "Energy".into(),
            change_pct: -0.45,
        },
        SectorPerformance {
            sector: "Financial Services".into(),
            change_pct: 0.78,
        },
    ];
    upsert_sector_performance(&c, &rows).unwrap();
    let got = get_sector_performance(&c).unwrap().unwrap();
    assert_eq!(got.len(), 3);
    assert_eq!(got[0].sector, "Technology");
    assert!(got[1].change_pct < 0.0);
}

#[test]
fn wacc_compute_basic_calc() {
    let s = compute_wacc_snapshot(
        "AAPL",
        "2026-04-14",
        1.20,                // beta
        3_000_000_000_000.0, // market cap (3T)
        4.50,                // Rf %
        100_000_000_000.0,   // total debt (100B)
        5_000_000_000.0,     // interest expense (5B)
        16.0,                // effective tax rate %
    );
    // Cost of equity = 4.5 + 1.20 * 5.0 = 10.5 %
    assert!((s.cost_of_equity_pct - 10.5).abs() < 1e-6);
    // Pre-tax cost of debt = (5B / 100B) * 100 = 5.0 %
    assert!((s.pre_tax_cost_of_debt_pct - 5.0).abs() < 1e-6);
    // After-tax = 5.0 * (1 - 0.16) = 4.2 %
    assert!((s.after_tax_cost_of_debt_pct - 4.2).abs() < 1e-6);
    // Weights: E=3T / (3T+100B) ≈ 0.9677, D ≈ 0.0323
    assert!((s.equity_weight - 3000.0 / 3100.0).abs() < 1e-6);
    // WACC ≈ 0.9677*10.5 + 0.0323*4.2 ≈ 10.296
    let expected = (3000.0 / 3100.0) * 10.5 + (100.0 / 3100.0) * 4.2;
    assert!((s.wacc_pct - expected).abs() < 1e-6);
}

#[test]
fn wacc_handles_zero_debt() {
    let s = compute_wacc_snapshot(
        "NVDA",
        "2026-04-14",
        1.80,                // beta
        2_500_000_000_000.0, // market cap
        4.30,                // Rf
        0.0,                 // no debt
        0.0,                 // no interest expense
        12.0,                // tax
    );
    assert_eq!(s.pre_tax_cost_of_debt_pct, 0.0);
    assert_eq!(s.debt_weight, 0.0);
    assert!((s.equity_weight - 1.0).abs() < 1e-9);
    // WACC == Re when all equity
    assert!((s.wacc_pct - s.cost_of_equity_pct).abs() < 1e-9);
}

#[test]
fn wacc_roundtrip() {
    let c = open_mem_conn_v6();
    let snap = compute_wacc_snapshot(
        "AAPL",
        "2026-04-14",
        1.20,
        3_000_000_000_000.0,
        4.50,
        100_000_000_000.0,
        5_000_000_000.0,
        16.0,
    );
    upsert_wacc(&c, "AAPL", &snap).unwrap();
    let got = get_wacc(&c, "aapl").unwrap().unwrap();
    assert_eq!(got.symbol, "AAPL");
    assert!((got.wacc_pct - snap.wacc_pct).abs() < 1e-9);
    assert!((got.beta - 1.20).abs() < 1e-9);
}

#[test]
fn fmp_mover_parses_string_percentage() {
    // FMP sometimes returns changesPercentage as "1.23%" (string), sometimes as f64.
    let v: serde_json::Value = serde_json::from_str(
        r#"{
            "symbol":"AAPL","name":"Apple","price":185.5,"change":2.1,
            "changesPercentage":"1.15%","volume":45000000
        }"#,
    )
    .unwrap();
    let m = parse_fmp_mover(&v);
    assert_eq!(m.symbol, "AAPL");
    assert!((m.change_pct - 1.15).abs() < 1e-9);
    assert!((m.volume - 45_000_000.0).abs() < 1.0);
}

// ── Research section ──

fn open_mem_conn_v7() -> Connection {
    let c = Connection::open_in_memory().unwrap();
    create_research_tables_v7(&c).unwrap();
    c
}

#[test]
fn fx_majors_universe_has_regions() {
    let regions: std::collections::HashSet<&str> = FX_MAJORS_UNIVERSE
        .iter()
        .map(|(_, _, _, _, r)| *r)
        .collect();
    assert!(regions.contains("Majors"));
    assert!(regions.contains("Crosses"));
    assert!(regions.contains("EM"));
}

#[test]
fn fx_majors_universe_has_eurusd_and_usdjpy() {
    let tickers: std::collections::HashSet<&str> = FX_MAJORS_UNIVERSE
        .iter()
        .map(|(t, _, _, _, _)| *t)
        .collect();
    assert!(tickers.contains("EURUSD=X"));
    assert!(tickers.contains("USDJPY=X"));
    assert!(tickers.contains("USDMXN=X"));
}

#[test]
fn currency_rates_roundtrip() {
    let c = open_mem_conn_v7();
    let rows = vec![
        CurrencyRate {
            ticker: "EURUSD=X".into(),
            display: "EUR/USD".into(),
            base: "EUR".into(),
            quote: "USD".into(),
            region: "Majors".into(),
            price: 1.0850,
            change: 0.0020,
            change_pct: 0.18,
        },
        CurrencyRate {
            ticker: "USDJPY=X".into(),
            display: "USD/JPY".into(),
            base: "USD".into(),
            quote: "JPY".into(),
            region: "Majors".into(),
            price: 151.25,
            change: -0.35,
            change_pct: -0.23,
        },
    ];
    upsert_currency_rates(&c, &rows).unwrap();
    let got = get_currency_rates(&c).unwrap().unwrap();
    assert_eq!(got.len(), 2);
    assert_eq!(got[0].display, "EUR/USD");
    assert!(got[1].change < 0.0);
}

#[test]
fn ols_regression_perfect_correlation() {
    // If s_i == 2 * m_i exactly, beta should be exactly 2.0, R² = 1.
    let m: Vec<f64> = vec![
        0.01, -0.005, 0.02, -0.01, 0.015, 0.008, -0.003, 0.012, 0.005, -0.007, 0.018, -0.002,
    ];
    let s: Vec<f64> = m.iter().map(|x| 2.0 * x).collect();
    let (beta, _alpha, r2, corr, n) = ols_regression(&s, &m);
    assert!((beta - 2.0).abs() < 1e-9);
    assert!((r2 - 1.0).abs() < 1e-9);
    assert!((corr - 1.0).abs() < 1e-9);
    assert_eq!(n, 12);
}

#[test]
fn compute_beta_snapshot_synthetic_2x_market() {
    // Build symbol bars that exactly track 2× market moves. Expected β ≈ 2.0.
    // Use 300 bars so the 1Y window (252) fits with headroom. FMP order is
    // newest-first, so we build newest → oldest. Dates must be unique —
    // we use a simple days-since-epoch counter so the join by date key
    // does not collide.
    let mut sym_bars: Vec<HistoricalPriceRow> = Vec::new();
    let mut mkt_bars: Vec<HistoricalPriceRow> = Vec::new();
    let mut sym_close = 100.0_f64;
    let mut mkt_close = 400.0_f64;
    for i in 0..300 {
        let daily = 0.01 * ((i as f64 * 0.37).sin());
        mkt_close *= 1.0 + daily;
        sym_close *= 1.0 + 2.0 * daily;
        // Fake-but-unique ISO date: walk calendar by 1-day increments from 2024-01-01.
        let base_day = 1 + (i % 28); // 1..=28
        let month = 1 + ((i / 28) % 12); // 1..=12
        let year = 2024 + ((i / (28 * 12)) as i32);
        let date = format!("{:04}-{:02}-{:02}", year, month, base_day);
        sym_bars.push(HistoricalPriceRow {
            date: date.clone(),
            open: sym_close,
            high: sym_close,
            low: sym_close,
            close: sym_close,
            adj_close: sym_close,
            volume: 0.0,
            change: 0.0,
            change_pct: 0.0,
        });
        mkt_bars.push(HistoricalPriceRow {
            date,
            open: mkt_close,
            high: mkt_close,
            low: mkt_close,
            close: mkt_close,
            adj_close: mkt_close,
            volume: 0.0,
            change: 0.0,
            change_pct: 0.0,
        });
    }
    // The loop already pushes in synthetic chronological order — we need
    // FMP's newest-first orientation, so reverse.
    sym_bars.reverse();
    mkt_bars.reverse();
    let snap = compute_beta_snapshot("TST", "SPY", "2026-04-14", &sym_bars, &mkt_bars);
    assert!(!snap.windows.is_empty());
    let w1y = snap
        .windows
        .iter()
        .find(|w| w.window_label == "1Y")
        .unwrap();
    assert!((w1y.beta - 2.0).abs() < 0.01, "beta was {}", w1y.beta);
    assert!(w1y.r_squared > 0.99);
}

#[test]
fn beta_snapshot_roundtrip() {
    let c = open_mem_conn_v7();
    let snap = BetaSnapshot {
        symbol: "AAPL".into(),
        market_ticker: "SPY".into(),
        as_of: "2026-04-14".into(),
        windows: vec![
            BetaWindow {
                window_label: "1Y".into(),
                window_days: 252,
                beta: 1.18,
                alpha_pct: 2.4,
                r_squared: 0.67,
                n_observations: 252,
                correlation: 0.82,
            },
            BetaWindow {
                window_label: "5Y".into(),
                window_days: 1260,
                beta: 1.23,
                alpha_pct: 4.1,
                r_squared: 0.71,
                n_observations: 1260,
                correlation: 0.84,
            },
        ],
        note: String::new(),
    };
    upsert_beta(&c, "AAPL", &snap).unwrap();
    let got = get_beta(&c, "aapl").unwrap().unwrap();
    assert_eq!(got.symbol, "AAPL");
    assert_eq!(got.windows.len(), 2);
    assert!((got.windows[0].beta - 1.18).abs() < 1e-9);
}

#[test]
fn compute_ddm_basic_growth() {
    // 10 years of dividends with 7% annual growth, required return 12% → finite price.
    let mut divs: Vec<DividendRecord> = Vec::new();
    let base = 1.00_f64;
    for y in 2016..=2025 {
        let growth = 1.07_f64.powi(y - 2016);
        for q in 1..=4 {
            divs.push(DividendRecord {
                ex_date: format!("{}-{:02}-15", y, 1 + (q - 1) * 3),
                pay_date: format!("{}-{:02}-28", y, 1 + (q - 1) * 3),
                record_date: String::new(),
                declaration_date: String::new(),
                amount: base * growth * 0.25,
                adjusted_amount: base * growth * 0.25,
                label: "Regular Cash".into(),
            });
        }
    }
    // Newest-first: sort descending by ex_date.
    divs.sort_by(|a, b| b.ex_date.cmp(&a.ex_date));
    let snap = compute_ddm_snapshot("AAA", "2026-04-14", &divs, 12.0, "WACC 12%");
    assert!(snap.annual_dividend > 0.0);
    assert!(
        snap.implied_growth_pct > 4.0 && snap.implied_growth_pct < 10.0,
        "growth was {}",
        snap.implied_growth_pct
    );
    assert!(snap.implied_price > 0.0);
    assert!(snap.note.is_empty());
}

#[test]
fn compute_ddm_diverges_when_growth_exceeds_return() {
    let divs = vec![
        DividendRecord {
            ex_date: "2025-01-15".into(),
            amount: 1.0,
            adjusted_amount: 1.0,
            ..Default::default()
        },
        DividendRecord {
            ex_date: "2024-01-15".into(),
            amount: 0.80,
            adjusted_amount: 0.80,
            ..Default::default()
        },
        DividendRecord {
            ex_date: "2023-01-15".into(),
            amount: 0.60,
            adjusted_amount: 0.60,
            ..Default::default()
        },
        DividendRecord {
            ex_date: "2022-01-15".into(),
            amount: 0.45,
            adjusted_amount: 0.45,
            ..Default::default()
        },
    ];
    // Ask for very low required return — Gordon must diverge.
    let snap = compute_ddm_snapshot("BBB", "2026-04-14", &divs, 2.0, "manual");
    assert_eq!(snap.implied_price, 0.0);
    assert!(!snap.note.is_empty());
}

#[test]
fn ddm_roundtrip() {
    let c = open_mem_conn_v7();
    let snap = DdmSnapshot {
        symbol: "KO".into(),
        as_of: "2026-04-14".into(),
        annual_dividend: 1.92,
        implied_growth_pct: 4.5,
        required_return_pct: 8.0,
        growth_source: "5Y dividend CAGR".into(),
        return_source: "WACC 8.0%".into(),
        implied_price: 57.34,
        method: "Gordon Growth".into(),
        note: String::new(),
    };
    upsert_ddm(&c, "KO", &snap).unwrap();
    let got = get_ddm(&c, "ko").unwrap().unwrap();
    assert_eq!(got.symbol, "KO");
    assert!((got.implied_price - 57.34).abs() < 1e-9);
}

#[test]
fn compute_relative_valuation_z_scores() {
    let inputs = vec![
        RvMetricInput {
            metric: "P/E",
            value: Some(30.0),
            peer_values: vec![10.0, 15.0, 20.0, 25.0, 28.0, 35.0, 40.0],
        },
        RvMetricInput {
            metric: "P/B",
            value: None, // should skip
            peer_values: vec![1.0, 2.0, 3.0, 4.0],
        },
        RvMetricInput {
            metric: "EV/EBITDA",
            value: Some(12.0),
            peer_values: vec![8.0, 10.0], // <3 peers — should skip
        },
    ];
    let rv = compute_relative_valuation("SUBJ", "Tech", "2026-04-14", &inputs);
    assert_eq!(rv.rows.len(), 1);
    let pe = &rv.rows[0];
    assert_eq!(pe.metric, "P/E");
    assert_eq!(pe.peer_low, 10.0);
    assert_eq!(pe.peer_high, 40.0);
    // 30 is higher than 5 of 7 peers → percentile ≈ 71.4
    assert!(pe.percentile > 60.0 && pe.percentile < 80.0);
    assert!(pe.z_score > 0.0); // above mean
}

#[test]
fn relative_valuation_roundtrip() {
    let c = open_mem_conn_v7();
    let rv = RelativeValuation {
        symbol: "AAPL".into(),
        sector: "Technology".into(),
        as_of: "2026-04-14".into(),
        peer_count: 8,
        rows: vec![RvMetricRow {
            metric: "P/E".into(),
            value: 32.0,
            peer_median: 28.0,
            peer_low: 12.0,
            peer_high: 60.0,
            z_score: 0.4,
            percentile: 62.5,
        }],
    };
    upsert_relative_valuation(&c, "AAPL", &rv).unwrap();
    let got = get_relative_valuation(&c, "aapl").unwrap().unwrap();
    assert_eq!(got.symbol, "AAPL");
    assert_eq!(got.rows.len(), 1);
    assert!((got.rows[0].value - 32.0).abs() < 1e-9);
}

#[test]
fn figi_roundtrip() {
    let c = open_mem_conn_v7();
    let snap = FigiSnapshot {
        symbol: "AAPL".into(),
        as_of: "2026-04-14".into(),
        identifiers: vec![FigiIdentifier {
            figi: "BBG000B9XRY4".into(),
            name: "APPLE INC".into(),
            ticker: "AAPL".into(),
            exch_code: "US".into(),
            composite_figi: "BBG000B9Y5X2".into(),
            share_class_figi: "BBG001S5N8V8".into(),
            security_type: "Common Stock".into(),
            security_type_2: "Common Stock".into(),
            market_sector: "Equity".into(),
            security_description: "AAPL".into(),
        }],
    };
    upsert_figi(&c, "AAPL", &snap).unwrap();
    let got = get_figi(&c, "aapl").unwrap().unwrap();
    assert_eq!(got.symbol, "AAPL");
    assert_eq!(got.identifiers.len(), 1);
    assert_eq!(got.identifiers[0].figi, "BBG000B9XRY4");
}

// ── Research section ──

fn open_mem_conn_v8() -> Connection {
    let c = Connection::open_in_memory().unwrap();
    create_research_tables_v8(&c).unwrap();
    c
}

#[test]
fn hra_roundtrip() {
    let c = open_mem_conn_v8();
    let snap = HraSnapshot {
        symbol: "AAPL".into(),
        as_of: "2026-04-14".into(),
        last_close: 190.0,
        windows: vec![HraWindow {
            label: "1Y".into(),
            trading_days: 252,
            return_pct: 22.0,
            cagr_pct: 22.0,
            n_observations: 252,
        }],
        max_drawdown_pct: -15.5,
        drawdown_peak_date: "2025-10-01".into(),
        drawdown_trough_date: "2025-12-15".into(),
        volatility_annual_pct: 24.0,
        sharpe_ratio: 1.1,
        sortino_ratio: 1.4,
        calmar_ratio: 1.4,
        risk_free_pct: 4.5,
        note: String::new(),
    };
    upsert_hra(&c, "AAPL", &snap).unwrap();
    let got = get_hra(&c, "aapl").unwrap().unwrap();
    assert_eq!(got.symbol, "AAPL");
    assert_eq!(got.windows.len(), 1);
    assert!((got.max_drawdown_pct - (-15.5)).abs() < 1e-6);
}

#[test]
fn dcf_roundtrip() {
    let c = open_mem_conn_v8();
    let snap = DcfSnapshot {
        symbol: "NVDA".into(),
        as_of: "2026-04-14".into(),
        method: "DCF on FCFF".into(),
        base_revenue: 60_000.0,
        base_fcff: 24_000.0,
        growth_pct: 20.0,
        terminal_growth_pct: 3.0,
        wacc_pct: 9.0,
        tax_rate_pct: 15.0,
        fcff_margin_pct: 40.0,
        projection_years: 5,
        years: Vec::new(),
        pv_sum: 100_000.0,
        terminal_value: 500_000.0,
        pv_terminal: 350_000.0,
        enterprise_value: 450_000.0,
        total_debt: 10_000.0,
        cash_and_equivalents: 30_000.0,
        equity_value: 470_000.0,
        shares_outstanding: 2_500.0,
        implied_price: 188.0,
        note: String::new(),
    };
    upsert_dcf(&c, "NVDA", &snap).unwrap();
    let got = get_dcf(&c, "nvda").unwrap().unwrap();
    assert_eq!(got.symbol, "NVDA");
    assert!((got.implied_price - 188.0).abs() < 1e-6);
}

#[test]
fn svm_roundtrip() {
    let c = open_mem_conn_v8();
    let snap = SvmSnapshot {
        symbol: "MSFT".into(),
        as_of: "2026-04-14".into(),
        current_price: 420.0,
        rows: vec![SvmModelRow {
            model: "DCF on FCFF".into(),
            implied_price: 450.0,
            current_price: 420.0,
            upside_pct: 7.14,
            confidence: "medium".into(),
            source: "test".into(),
        }],
        fair_low: 450.0,
        fair_high: 450.0,
        fair_mid: 450.0,
        upside_mid_pct: 7.14,
        note: String::new(),
    };
    upsert_svm(&c, "MSFT", &snap).unwrap();
    let got = get_svm(&c, "msft").unwrap().unwrap();
    assert_eq!(got.rows.len(), 1);
    assert!((got.fair_mid - 450.0).abs() < 1e-6);
}

#[test]
fn options_chain_roundtrip() {
    let c = open_mem_conn_v8();
    let snap = OptionsChainSnapshot {
        symbol: "SPY".into(),
        as_of: "2026-04-14".into(),
        underlying_price: 520.0,
        expirations: vec![OptionExpiry {
            expiration: "2026-05-16".into(),
            days_to_expiry: 32,
            calls: vec![OptionContract {
                contract_symbol: "SPY260516C00520000".into(),
                option_type: "CALL".into(),
                strike: 520.0,
                last_price: 8.5,
                bid: 8.4,
                ask: 8.6,
                volume: 1200.0,
                open_interest: 5000.0,
                implied_volatility: 0.18,
                in_the_money: false,
            }],
            puts: vec![],
        }],
        note: String::new(),
    };
    upsert_options_chain(&c, "SPY", &snap).unwrap();
    let got = get_options_chain(&c, "spy").unwrap().unwrap();
    assert_eq!(got.expirations.len(), 1);
    assert_eq!(got.expirations[0].calls.len(), 1);
    assert!((got.expirations[0].calls[0].strike - 520.0).abs() < 1e-6);
}

#[test]
fn yahoo_option_expiry_parser_preserves_calls_and_puts() {
    let options = serde_json::json!({
        "expirationDate": 1_893_456_000i64,
        "calls": [{
            "contractSymbol": "SPY300101C00520000",
            "strike": 520.0,
            "lastPrice": 8.5,
            "bid": 8.4,
            "ask": 8.6,
            "volume": 1200.0,
            "openInterest": 5000.0,
            "impliedVolatility": 0.18
        }],
        "puts": [{
            "contractSymbol": "SPY300101P00520000",
            "strike": 520.0,
            "lastPrice": 7.5,
            "bid": 7.4,
            "ask": 7.6,
            "volume": 900.0,
            "openInterest": 4000.0,
            "impliedVolatility": 0.20
        }]
    });
    let expiry = parse_yahoo_option_expiry(&options, 525.0);
    assert_eq!(expiry.calls.len(), 1);
    assert_eq!(expiry.puts.len(), 1);
    assert!(expiry.calls[0].in_the_money);
    assert!(!expiry.puts[0].in_the_money);
    assert_eq!(expiry.calls[0].contract_symbol, "SPY300101C00520000");
}

#[test]
fn ivol_roundtrip() {
    let c = open_mem_conn_v8();
    let snap = IvolSnapshot {
        symbol: "TSLA".into(),
        as_of: "2026-04-14".into(),
        current_atm_iv_pct: 55.0,
        iv_52w_low_pct: 30.0,
        iv_52w_high_pct: 80.0,
        iv_rank: 50.0,
        iv_percentile: 60.0,
        observation_count: 100,
        history: vec![IvolObservation {
            date: "2026-01-01".into(),
            atm_iv_pct: 40.0,
        }],
        note: String::new(),
    };
    upsert_ivol(&c, "TSLA", &snap).unwrap();
    let got = get_ivol(&c, "tsla").unwrap().unwrap();
    assert!((got.iv_rank - 50.0).abs() < 1e-6);
    assert_eq!(got.history.len(), 1);
}

#[test]
fn compute_hra_on_synthetic_uptrend() {
    // 300 daily bars, +0.1% per day → terminal ~ 1.001^299.
    let mut bars: Vec<HistoricalPriceRow> = Vec::new();
    let mut px = 100.0;
    for i in 0..300 {
        let base_day = 1 + (i % 28);
        let month = 1 + ((i / 28) % 12);
        let year = 2024 + (i / (28 * 12));
        bars.push(HistoricalPriceRow {
            date: format!("{:04}-{:02}-{:02}", year, month, base_day),
            open: px,
            high: px,
            low: px,
            close: px,
            adj_close: px,
            volume: 1_000.0,
            change: 0.0,
            change_pct: 0.1,
        });
        px *= 1.001;
    }
    let snap = compute_hra_snapshot("TEST", "2026-04-14", &bars, 4.5);
    assert_eq!(snap.symbol, "TEST");
    // 1Y window present
    assert!(snap.windows.iter().any(|w| w.label == "1Y"));
    // ITD should be strongly positive
    let itd = snap.windows.iter().find(|w| w.label == "ITD").unwrap();
    assert!(itd.return_pct > 0.0);
    // Monotonic uptrend → drawdown effectively zero (we accept very small
    // rounding-scale negatives).
    assert!(
        snap.max_drawdown_pct > -0.1,
        "expected near-zero drawdown on monotonic uptrend, got {}",
        snap.max_drawdown_pct
    );
}

#[test]
fn compute_hra_on_empty_bars_returns_note() {
    let snap = compute_hra_snapshot("EMPTY", "2026-04-14", &[], 4.5);
    assert!(!snap.note.is_empty());
    assert_eq!(snap.windows.len(), 0);
}

#[test]
fn compute_hra_drawdown_detects_peak_and_trough() {
    // 50 bars that rise to 150 at day 20, then fall to 100 by day 40, then
    // recover to 130 at day 49. Max DD is from peak 150 to trough 100.
    let mut bars: Vec<HistoricalPriceRow> = Vec::new();
    let mut push = |i: usize, close: f64| {
        let base_day = 1 + (i % 28);
        let month = 1 + ((i / 28) % 12);
        let year = 2024 + (i / (28 * 12));
        bars.push(HistoricalPriceRow {
            date: format!("{:04}-{:02}-{:02}", year, month, base_day),
            open: close,
            high: close,
            low: close,
            close,
            adj_close: close,
            volume: 1_000.0,
            change: 0.0,
            change_pct: 0.0,
        });
    };
    for i in 0..20 {
        push(i, 100.0 + (i as f64 * 2.5));
    } // 100 → 147.5
    push(20, 150.0); // peak
    for i in 21..=40 {
        push(i, 150.0 - ((i - 20) as f64 * 2.5));
    } // 150 → 100
    for i in 41..50 {
        push(i, 100.0 + ((i - 40) as f64 * 3.333));
    } // 100 → 130
    let snap = compute_hra_snapshot("X", "2026-04-14", &bars, 0.0);
    // Peak-to-trough 150→100 = -33.33%
    assert!(
        snap.max_drawdown_pct < -32.0 && snap.max_drawdown_pct > -34.0,
        "expected ~-33% drawdown, got {:.2}",
        snap.max_drawdown_pct
    );
}

#[test]
fn compute_dcf_basic() {
    let snap = compute_dcf_snapshot(
        "NVDA",
        "2026-04-14",
        /*revenue*/ 60_000.0,
        /*fcff*/ 24_000.0,
        /*g*/ 20.0,
        /*tg*/ 3.0,
        /*wacc*/ 9.0,
        /*tax*/ 15.0,
        /*years*/ 5,
        /*debt*/ 10_000.0,
        /*cash*/ 30_000.0,
        /*shares*/ 2_500.0,
    );
    assert_eq!(snap.years.len(), 5);
    assert!(snap.enterprise_value > 0.0);
    assert!(snap.implied_price > 0.0);
    // Each projection year's fcff should compound
    assert!(snap.years[4].fcff > snap.years[0].fcff);
}

#[test]
fn compute_dcf_rejects_terminal_growth_above_wacc() {
    let snap = compute_dcf_snapshot(
        "X",
        "2026-04-14",
        100.0,
        40.0,
        5.0,
        15.0,
        8.0,
        20.0,
        5,
        10.0,
        5.0,
        100.0,
    );
    assert!(!snap.note.is_empty());
    assert_eq!(snap.implied_price, 0.0);
}

#[test]
fn compute_svm_triangulates_multiple_models() {
    let ddm = DdmSnapshot {
        symbol: "XYZ".into(),
        as_of: "2026-04-14".into(),
        annual_dividend: 3.0,
        implied_growth_pct: 4.0,
        required_return_pct: 10.0,
        growth_source: "test".into(),
        return_source: "test".into(),
        implied_price: 52.0,
        method: "Gordon Growth".into(),
        note: String::new(),
    };
    let dcf = DcfSnapshot {
        symbol: "XYZ".into(),
        as_of: "2026-04-14".into(),
        method: "DCF on FCFF".into(),
        base_revenue: 100.0,
        base_fcff: 20.0,
        growth_pct: 5.0,
        terminal_growth_pct: 2.0,
        wacc_pct: 10.0,
        tax_rate_pct: 20.0,
        fcff_margin_pct: 20.0,
        projection_years: 5,
        years: Vec::new(),
        pv_sum: 0.0,
        terminal_value: 0.0,
        pv_terminal: 0.0,
        enterprise_value: 0.0,
        total_debt: 0.0,
        cash_and_equivalents: 0.0,
        equity_value: 0.0,
        shares_outstanding: 1.0,
        implied_price: 58.0,
        note: String::new(),
    };
    let snap = compute_svm_snapshot(
        "XYZ",
        "2026-04-14",
        /*current*/ 50.0,
        Some(&ddm),
        Some(&dcf),
        Some((12.0, 4.5)),                 // P/E × EPS → 54
        Some((10.0, 10.0, 5.0, 2.0, 1.0)), // EV/EBITDA 10 × 10 → EV 100 - 5 + 2 = 97 / 1 shares = 97
        Some((1.2, 45.0)),                 // P/B × BVPS → 54
    );
    assert!(
        snap.rows.len() >= 4,
        "expected ≥4 triangulation rows, got {}",
        snap.rows.len()
    );
    assert!(snap.fair_low > 0.0);
    assert!(snap.fair_mid >= snap.fair_low);
    assert!(snap.fair_high >= snap.fair_mid);
    assert!(
        snap.upside_mid_pct > 0.0,
        "at $50 current vs mid, upside should be positive"
    );
}

#[test]
fn compute_svm_with_no_models_emits_note() {
    let snap = compute_svm_snapshot("X", "2026-04-14", 50.0, None, None, None, None, None);
    assert!(snap.rows.is_empty());
    assert!(!snap.note.is_empty());
}

#[test]
fn compute_ivol_rank_and_percentile() {
    let history: Vec<IvolObservation> = (0..100)
        .map(|i| IvolObservation {
            date: format!("2025-{:03}", i),
            atm_iv_pct: 20.0 + (i as f64 * 0.3),
        })
        .collect();
    // History spans 20% → 49.7%; current = 40%.
    let snap = compute_ivol_snapshot("TEST", "2026-04-14", 40.0, &history);
    // Rank: (40 - 20) / (49.7 - 20) × 100 ≈ 67
    assert!(
        snap.iv_rank > 50.0 && snap.iv_rank < 80.0,
        "expected rank 50-80, got {:.2}",
        snap.iv_rank
    );
    // Percentile: ~67% of observations ≤ 40
    assert!(
        snap.iv_percentile > 50.0 && snap.iv_percentile < 80.0,
        "expected percentile 50-80, got {:.2}",
        snap.iv_percentile
    );
}

#[test]
fn compute_ivol_with_no_history_uses_placeholder() {
    let snap = compute_ivol_snapshot("NEW", "2026-04-14", 25.0, &[]);
    assert!(!snap.note.is_empty());
    assert!((snap.iv_52w_low_pct - 25.0).abs() < 1e-6);
    assert!((snap.iv_52w_high_pct - 25.0).abs() < 1e-6);
}

// ── Research section ──

fn open_mem_conn_v9() -> Connection {
    let c = Connection::open_in_memory().unwrap();
    create_research_tables_v9(&c).unwrap();
    c
}

fn synth_bars(n: usize, start: f64, daily_drift: f64) -> Vec<HistoricalPriceRow> {
    let mut bars = Vec::with_capacity(n);
    let mut px = start;
    for i in 0..n {
        let base_day = 1 + (i % 28);
        let month = 1 + ((i / 28) % 12);
        let year = 2024 + (i / (28 * 12));
        bars.push(HistoricalPriceRow {
            date: format!("{:04}-{:02}-{:02}", year, month, base_day),
            open: px,
            high: px * 1.005,
            low: px * 0.995,
            close: px,
            adj_close: px,
            volume: 1_000.0,
            change: 0.0,
            change_pct: 0.0,
        });
        px *= 1.0 + daily_drift;
    }
    bars
}

#[test]
fn seasonality_snapshot_roundtrip() {
    let c = open_mem_conn_v9();
    let snap = SeasonalitySnapshot {
        symbol: "AAPL".into(),
        as_of: "2026-04-14".into(),
        years_covered: 3,
        months: vec![SeasonalityMonth {
            month: 1,
            label: "Jan".into(),
            avg_return_pct: 2.1,
            median_return_pct: 1.8,
            stdev_pct: 3.4,
            positive_years: 2,
            total_years: 3,
            best_return_pct: 5.1,
            worst_return_pct: -1.2,
        }],
        dow: vec![SeasonalityDow {
            dow: 1,
            label: "Mon".into(),
            avg_return_pct: 0.05,
            positive_days: 28,
            total_days: 52,
        }],
        best_month: "Jul".into(),
        worst_month: "Sep".into(),
        note: String::new(),
    };
    upsert_seasonality(&c, "AAPL", &snap).unwrap();
    let got = get_seasonality(&c, "aapl").unwrap().unwrap();
    assert_eq!(got.symbol, "AAPL");
    assert_eq!(got.months.len(), 1);
    assert_eq!(got.best_month, "Jul");
}

#[test]
fn correlation_matrix_roundtrip() {
    let c = open_mem_conn_v9();
    let snap = CorrelationMatrix {
        symbol: "AAPL".into(),
        as_of: "2026-04-14".into(),
        window_days: 252,
        cells: vec![CorrelationCell {
            peer_symbol: "MSFT".into(),
            correlation: 0.85,
            n_observations: 245,
            beta_vs_peer: 0.92,
        }],
        mean_correlation: 0.85,
        highest_corr_symbol: "MSFT".into(),
        lowest_corr_symbol: "MSFT".into(),
        note: String::new(),
    };
    upsert_correlation(&c, "AAPL", &snap).unwrap();
    let got = get_correlation(&c, "aapl").unwrap().unwrap();
    assert_eq!(got.cells.len(), 1);
    assert!((got.mean_correlation - 0.85).abs() < 1e-6);
}

#[test]
fn total_return_snapshot_roundtrip() {
    let c = open_mem_conn_v9();
    let snap = TotalReturnSnapshot {
        symbol: "KO".into(),
        as_of: "2026-04-14".into(),
        last_close: 60.0,
        trailing_12m_dividends: 1.84,
        trailing_12m_yield_pct: 3.07,
        windows: vec![TotalReturnWindow {
            label: "1Y".into(),
            trading_days: 252,
            price_return_pct: 8.0,
            dividend_yield_pct: 3.1,
            total_return_pct: 11.1,
            annualized_pct: 11.1,
            dividends_paid: 1.84,
            n_dividends: 4,
        }],
        note: String::new(),
    };
    upsert_total_return(&c, "KO", &snap).unwrap();
    let got = get_total_return(&c, "ko").unwrap().unwrap();
    assert_eq!(got.windows.len(), 1);
    assert!((got.trailing_12m_yield_pct - 3.07).abs() < 1e-6);
}

#[test]
fn technicals_snapshot_roundtrip() {
    let c = open_mem_conn_v9();
    let snap = TechnicalSnapshot {
        symbol: "NVDA".into(),
        as_of: "2026-04-14".into(),
        last_close: 850.0,
        indicators: vec![TechnicalIndicator {
            name: "RSI(14)".into(),
            value: 72.5,
            value_secondary: 0.0,
            value_tertiary: 0.0,
            signal: "overbought".into(),
            note: String::new(),
        }],
        trend_summary: "bullish composite".into(),
        note: String::new(),
    };
    upsert_technicals(&c, "NVDA", &snap).unwrap();
    let got = get_technicals(&c, "nvda").unwrap().unwrap();
    assert_eq!(got.indicators.len(), 1);
    assert_eq!(got.trend_summary, "bullish composite");
}

#[test]
fn vol_skew_roundtrip() {
    let c = open_mem_conn_v9();
    let snap = VolatilitySkew {
        symbol: "SPY".into(),
        as_of: "2026-04-14".into(),
        underlying_price: 520.0,
        expiries: vec![SkewExpiry {
            expiration: "2026-05-16".into(),
            days_to_expiry: 32,
            atm_iv_pct: 18.5,
            points: vec![SkewPoint {
                strike: 520.0,
                moneyness_pct: 0.0,
                call_iv_pct: 18.3,
                put_iv_pct: 18.7,
                combined_iv_pct: 18.5,
            }],
            put_call_skew_25d_pct: 2.1,
            term_note: String::new(),
        }],
        note: String::new(),
    };
    upsert_vol_skew(&c, "SPY", &snap).unwrap();
    let got = get_vol_skew(&c, "spy").unwrap().unwrap();
    assert_eq!(got.expiries.len(), 1);
    assert_eq!(got.expiries[0].points.len(), 1);
}

#[test]
fn compute_seasonality_on_monthly_uptrend() {
    // 2 full years × 12 months × 21 bars = 504 bars.
    // Deterministic upward drift so every month is positive.
    let bars = synth_bars(504, 100.0, 0.001);
    let snap = compute_seasonality_snapshot("TEST", "2026-04-14", &bars);
    assert_eq!(snap.symbol, "TEST");
    assert!(snap.years_covered >= 2);
    assert!(snap.months.iter().any(|m| m.total_years > 0));
    // With uniform positive drift the best month should have a positive mean.
    let best = snap
        .months
        .iter()
        .max_by(|a, b| a.avg_return_pct.partial_cmp(&b.avg_return_pct).unwrap())
        .unwrap();
    assert!(best.avg_return_pct > 0.0);
}

#[test]
fn compute_seasonality_on_empty_returns_note() {
    let snap = compute_seasonality_snapshot("X", "2026-04-14", &[]);
    assert!(!snap.note.is_empty());
    assert_eq!(snap.years_covered, 0);
}

#[test]
fn compute_correlation_matrix_perfect_copy() {
    // Bars need variable returns — constant drift produces zero variance
    // and an undefined ρ (our compute treats this as 0).
    let mut bars: Vec<HistoricalPriceRow> = Vec::new();
    let mut px = 100.0;
    for i in 0..300 {
        let base_day = 1 + (i % 28);
        let month = 1 + ((i / 28) % 12);
        let year = 2024 + (i / (28 * 12));
        let drift = if i % 2 == 0 { 0.005 } else { -0.003 };
        bars.push(HistoricalPriceRow {
            date: format!("{:04}-{:02}-{:02}", year, month, base_day),
            open: px,
            high: px * 1.01,
            low: px * 0.99,
            close: px,
            adj_close: px,
            volume: 1_000.0,
            change: 0.0,
            change_pct: 0.0,
        });
        px *= 1.0 + drift;
    }
    let peer = bars.clone();
    let snap = compute_correlation_matrix("A", "2026-04-14", 252, &bars, &[("B".into(), peer)]);
    assert_eq!(snap.cells.len(), 1);
    // Perfect copy ⇒ correlation ≈ 1.0 (allow numerical slack).
    assert!(
        snap.cells[0].correlation > 0.999,
        "expected ρ≈1.0, got {}",
        snap.cells[0].correlation
    );
    assert!((snap.cells[0].beta_vs_peer - 1.0).abs() < 1e-6);
}

#[test]
fn compute_correlation_matrix_skips_empty_peers() {
    let bars = synth_bars(300, 100.0, 0.001);
    let snap =
        compute_correlation_matrix("A", "2026-04-14", 252, &bars, &[("NO_DATA".into(), vec![])]);
    assert!(!snap.note.is_empty() || snap.cells.is_empty());
}

#[test]
fn compute_total_return_with_dividends_sums_windows() {
    // synth_bars(260, ...) spans 2024-01-01 through roughly 2024-10-08, so
    // dividend ex-dates must live inside that window to be counted.
    let bars = synth_bars(260, 100.0, 0.0004);
    let divs: Vec<DividendRecord> = vec![
        DividendRecord {
            ex_date: "2024-03-15".into(),
            amount: 0.5,
            ..Default::default()
        },
        DividendRecord {
            ex_date: "2024-06-15".into(),
            amount: 0.5,
            ..Default::default()
        },
        DividendRecord {
            ex_date: "2024-09-15".into(),
            amount: 0.5,
            ..Default::default()
        },
    ];
    let snap = compute_total_return_snapshot("TEST", "2024-10-15", &bars, &divs);
    assert!(snap.windows.iter().any(|w| w.label == "1Y"));
    // At least one window should record some dividends paid.
    assert!(snap.windows.iter().any(|w| w.dividends_paid > 0.0));
}

#[test]
fn compute_technical_indicators_on_uptrend_is_bullish() {
    let bars = synth_bars(120, 100.0, 0.002);
    let snap = compute_technical_indicators("TEST", "2026-04-14", &bars);
    assert!(!snap.indicators.is_empty());
    // RSI on a steady uptrend should bias above 50 (often into overbought).
    let rsi = snap
        .indicators
        .iter()
        .find(|i| i.name.starts_with("RSI"))
        .unwrap();
    assert!(
        rsi.value > 50.0,
        "expected RSI > 50 on uptrend, got {:.2}",
        rsi.value
    );
}

#[test]
fn compute_technical_indicators_insufficient_bars_returns_note() {
    let bars = synth_bars(10, 100.0, 0.001);
    let snap = compute_technical_indicators("X", "2026-04-14", &bars);
    assert!(!snap.note.is_empty());
    assert!(snap.indicators.is_empty());
}

#[test]
fn compute_volatility_skew_basic_smile() {
    let chain = OptionsChainSnapshot {
        symbol: "SPY".into(),
        as_of: "2026-04-14".into(),
        underlying_price: 500.0,
        expirations: vec![OptionExpiry {
            expiration: "2026-05-16".into(),
            days_to_expiry: 32,
            calls: vec![
                OptionContract {
                    strike: 450.0,
                    option_type: "CALL".into(),
                    implied_volatility: 0.23,
                    in_the_money: true,
                    ..Default::default()
                },
                OptionContract {
                    strike: 500.0,
                    option_type: "CALL".into(),
                    implied_volatility: 0.18,
                    in_the_money: false,
                    ..Default::default()
                },
                OptionContract {
                    strike: 550.0,
                    option_type: "CALL".into(),
                    implied_volatility: 0.21,
                    in_the_money: false,
                    ..Default::default()
                },
            ],
            puts: vec![
                OptionContract {
                    strike: 450.0,
                    option_type: "PUT".into(),
                    implied_volatility: 0.25,
                    in_the_money: false,
                    ..Default::default()
                },
                OptionContract {
                    strike: 500.0,
                    option_type: "PUT".into(),
                    implied_volatility: 0.19,
                    in_the_money: false,
                    ..Default::default()
                },
                OptionContract {
                    strike: 550.0,
                    option_type: "PUT".into(),
                    implied_volatility: 0.20,
                    in_the_money: true,
                    ..Default::default()
                },
            ],
        }],
        note: String::new(),
    };
    let snap = compute_volatility_skew("SPY", "2026-04-14", &chain);
    assert_eq!(snap.expiries.len(), 1);
    let e = &snap.expiries[0];
    assert_eq!(e.points.len(), 3);
    // ATM (500) IV should be lowest (smile).
    assert!(e.atm_iv_pct > 0.0);
    // OTM put (450) IV 25% > OTM call (550) IV 21% → positive skew.
    assert!(
        e.put_call_skew_25d_pct > 0.0,
        "expected positive skew, got {}",
        e.put_call_skew_25d_pct
    );
}

#[test]
fn compute_volatility_skew_empty_chain_returns_note() {
    let chain = OptionsChainSnapshot {
        symbol: "X".into(),
        as_of: "2026-04-14".into(),
        underlying_price: 100.0,
        expirations: Vec::new(),
        note: String::new(),
    };
    let snap = compute_volatility_skew("X", "2026-04-14", &chain);
    assert!(!snap.note.is_empty());
}
