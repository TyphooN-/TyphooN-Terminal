use super::*;
use rusqlite::Connection;

fn setup_test_db() -> Connection {
    let conn = Connection::open_in_memory().unwrap();
    create_fundamentals_tables(&conn).unwrap();
    conn
}

// ── format_large_number ────────────────────────────────────────

#[test]
fn format_large_number_trillions() {
    assert_eq!(format_large_number(1_500_000_000_000.0), "$1.50T");
    assert_eq!(format_large_number(2_000_000_000_000.0), "$2.00T");
}

#[test]
fn format_large_number_billions() {
    assert_eq!(format_large_number(3_450_000_000.0), "$3.45B");
    assert_eq!(format_large_number(999_999_999.99), "$1000.00M"); // just under 1B
}

#[test]
fn format_large_number_millions() {
    assert_eq!(format_large_number(12_340_000.0), "$12.34M");
    assert_eq!(format_large_number(1_000_000.0), "$1.00M");
}

#[test]
fn format_large_number_thousands() {
    assert_eq!(format_large_number(5_000.0), "$5.0K");
    assert_eq!(format_large_number(1_234.5), "$1.2K");
}

#[test]
fn format_large_number_small() {
    assert_eq!(format_large_number(42.5), "$42.50");
    assert_eq!(format_large_number(0.99), "$0.99");
}

#[test]
fn format_large_number_negative() {
    // Negative values should use the same thresholds based on abs
    assert_eq!(format_large_number(-2_000_000_000.0), "$-2.00B");
    assert_eq!(format_large_number(-500.0), "$-500.00");
}

// ── extract_usd_fact ───────────────────────────────────────────

#[test]
fn extract_usd_fact_picks_most_recent() {
    let json = serde_json::json!({
        "facts": {
            "us-gaap": {
                "CashAndCashEquivalentsAtCarryingValue": {
                    "units": {
                        "USD": [
                            { "end": "2023-12-31", "val": 10000.0 },
                            { "end": "2024-06-30", "val": 25000.0 },
                            { "end": "2024-03-31", "val": 15000.0 }
                        ]
                    }
                }
            }
        }
    });
    let result = extract_usd_fact(&json, "CashAndCashEquivalentsAtCarryingValue");
    assert_eq!(result, Some(25000.0));
}

#[test]
fn extract_usd_fact_missing_concept() {
    let json = serde_json::json!({ "facts": { "us-gaap": {} } });
    assert_eq!(extract_usd_fact(&json, "NonExistent"), None);
}

#[test]
fn extract_usd_fact_empty_array() {
    let json = serde_json::json!({
        "facts": { "us-gaap": { "LongTermDebt": { "units": { "USD": [] } } } }
    });
    assert_eq!(extract_usd_fact(&json, "LongTermDebt"), None);
}

// ── yahoo_json_raw / yahoo_json_fmt ────────────────────────────────────────────

#[test]
fn yf_raw_extracts_raw_value() {
    let json = serde_json::json!({
        "price": { "marketCap": { "raw": 1234567890.0, "fmt": "1.23B" } }
    });
    assert_eq!(
        yahoo_json_raw(&json, "/price/marketCap"),
        Some(1234567890.0)
    );
}

#[test]
fn yf_raw_returns_none_on_missing() {
    let json = serde_json::json!({ "price": {} });
    assert_eq!(yahoo_json_raw(&json, "/price/marketCap"), None);
}

#[test]
fn yf_fmt_extracts_formatted_string() {
    let json = serde_json::json!({
        "calendarEvents": { "exDividendDate": { "raw": 1718409600, "fmt": "2024-06-15" } }
    });
    assert_eq!(
        yahoo_json_fmt(&json, "/calendarEvents/exDividendDate"),
        Some("2024-06-15".to_string())
    );
}

#[test]
fn yf_fmt_returns_none_on_missing() {
    let json = serde_json::json!({});
    assert_eq!(
        yahoo_json_fmt(&json, "/calendarEvents/exDividendDate"),
        None
    );
}

// ── parse_yahoo_data ───────────────────────────────────────────

#[test]
fn parse_yahoo_data_basic_fields() {
    let yahoo = serde_json::json!({
        "summaryProfile": {
            "sector": "Technology",
            "industry": "Semiconductors",
            "longBusinessSummary": "A chip company."
        },
        "price": {
            "shortName": "ACME Corp",
            "marketCap": { "raw": 50_000_000_000.0 },
            "regularMarketPrice": { "raw": 150.0 }
        },
        "defaultKeyStatistics": {
            "enterpriseValue": { "raw": 55_000_000_000.0 },
            "sharesOutstanding": { "raw": 333_333_333.0 },
            "beta": { "raw": 1.2 }
        }
    });

    let f = parse_yahoo_data("ACME", &yahoo);
    assert_eq!(f.symbol, "ACME");
    assert_eq!(f.company_name, "ACME Corp");
    assert_eq!(f.sector, "Technology");
    assert_eq!(f.industry, "Semiconductors");
    assert_eq!(f.description, "A chip company.");
    assert_eq!(f.market_cap, Some(50_000_000_000.0));
    assert_eq!(f.stock_price, Some(150.0));
    assert_eq!(f.enterprise_value, Some(55_000_000_000.0));
    assert_eq!(f.beta, Some(1.2));
}

#[test]
fn parse_yahoo_data_ev_fallback_calculation() {
    // If enterpriseValue is missing, it should be calculated from mcap + debt - cash
    let yahoo = serde_json::json!({
        "price": {
            "marketCap": { "raw": 100_000.0 },
            "regularMarketPrice": { "raw": 10.0 }
        },
        "financialData": {
            "totalDebt": { "raw": 30_000.0 },
            "totalCash": { "raw": 5_000.0 }
        }
    });

    let f = parse_yahoo_data("TEST", &yahoo);
    assert_eq!(f.enterprise_value, Some(125_000.0)); // 100k + 30k - 5k
    // mcap_ev_ratio = 100k / 125k * 100 = 80.0
    let ratio = f.mcap_ev_ratio.unwrap();
    assert!((ratio - 80.0).abs() < 0.01);
}

#[test]
fn parse_yahoo_data_etf_fallback() {
    // ETFs return an empty summaryProfile — sector/industry must fall back to
    // fundProfile.categoryName + quoteType.quoteType = "ETF".
    let yahoo = serde_json::json!({
        "quoteType": { "quoteType": "ETF" },
        "fundProfile": {
            "family": "iShares",
            "categoryName": "Large Blend",
            "legalType": "Exchange Traded Fund"
        },
        "price": {
            "shortName": "iShares Core S&P 500 ETF",
            "marketCap": { "raw": 500_000_000_000.0 },
            "regularMarketPrice": { "raw": 500.0 }
        }
    });

    let f = parse_yahoo_data("IVV", &yahoo);
    assert_eq!(f.symbol, "IVV");
    assert_eq!(f.sector, "ETF");
    assert_eq!(f.industry, "Large Blend");
    assert!(f.description.contains("iShares"));
    assert!(f.description.contains("Large Blend"));
}

#[test]
fn parse_yahoo_data_mutual_fund_fallback() {
    // Mutual funds use the same fundProfile path but with a different sector bucket.
    let yahoo = serde_json::json!({
        "quoteType": { "quoteType": "MUTUALFUND" },
        "fundProfile": {
            "family": "Vanguard",
            "categoryName": "Emerging Markets",
            "legalType": "Open End Fund"
        }
    });
    let f = parse_yahoo_data("VEMAX", &yahoo);
    assert_eq!(f.sector, "Mutual Fund");
    assert_eq!(f.industry, "Emerging Markets");
}

#[test]
fn parse_yahoo_data_quotetype_last_resort() {
    // When fundProfile is absent but quoteType identifies a non-equity instrument,
    // sector should still get a meaningful bucket.
    let yahoo = serde_json::json!({
        "quoteType": { "quoteType": "CRYPTOCURRENCY" }
    });
    let f = parse_yahoo_data("BTC-USD", &yahoo);
    assert_eq!(f.sector, "Crypto");
}

#[test]
fn parse_yahoo_data_equity_unchanged_by_fallback() {
    // A regular equity with a populated summaryProfile must NOT get overwritten
    // by the ETF fallback branch.
    let yahoo = serde_json::json!({
        "summaryProfile": {
            "sector": "Technology",
            "industry": "Semiconductors"
        },
        "quoteType": { "quoteType": "EQUITY" },
        "fundProfile": { "categoryName": "SHOULD NOT BE USED" }
    });
    let f = parse_yahoo_data("NVDA", &yahoo);
    assert_eq!(f.sector, "Technology");
    assert_eq!(f.industry, "Semiconductors");
}

#[test]
fn parse_yahoo_data_dividend_stock() {
    let yahoo = serde_json::json!({
        "summaryDetail": {
            "dividendRate": { "raw": 2.5 },
            "dividendYield": { "raw": 0.015 }
        }
    });

    let f = parse_yahoo_data("DIV", &yahoo);
    assert!(f.is_dividend_stock);
    assert_eq!(f.dividend_yield, Some(0.015));
}

// ── parse_quarterly_financials ─────────────────────────────────

#[test]
fn parse_quarterly_financials_basic() {
    let yahoo = serde_json::json!({
        "incomeStatementHistoryQuarterly": {
            "incomeStatementHistory": [
                {
                    "endDate": { "raw": 1711843200, "fmt": "2024-03-31" },
                    "totalRevenue": { "raw": 50_000_000.0 },
                    "netIncome": { "raw": 5_000_000.0 },
                    "grossProfit": { "raw": 20_000_000.0 },
                    "operatingIncome": { "raw": 8_000_000.0 }
                }
            ]
        }
    });

    let quarters = parse_quarterly_financials("TEST", &yahoo);
    assert_eq!(quarters.len(), 1);
    assert_eq!(quarters[0].symbol, "TEST");
    assert_eq!(quarters[0].period_end, "2024-03-31");
    assert_eq!(quarters[0].total_revenue, Some(50_000_000.0));
    assert_eq!(quarters[0].net_income, Some(5_000_000.0));
}

#[test]
fn parse_quarterly_financials_with_cashflow() {
    let yahoo = serde_json::json!({
        "incomeStatementHistoryQuarterly": {
            "incomeStatementHistory": [
                {
                    "endDate": { "raw": 1711843200, "fmt": "2024-03-31" },
                    "totalRevenue": { "raw": 100_000.0 },
                    "netIncome": { "raw": 10_000.0 }
                }
            ]
        },
        "cashflowStatementHistoryQuarterly": {
            "cashflowStatements": [
                {
                    "endDate": { "raw": 1711843200, "fmt": "2024-03-31" },
                    "totalCashFromOperatingActivities": { "raw": 15_000.0 },
                    "capitalExpenditures": { "raw": -3_000.0 }
                }
            ]
        }
    });

    let quarters = parse_quarterly_financials("TEST", &yahoo);
    assert_eq!(quarters.len(), 1);
    // FCF = operating CF - |capex| = 15000 - 3000 = 12000
    assert_eq!(quarters[0].free_cash_flow, Some(12_000.0));
}

#[test]
fn parse_quarterly_financials_empty() {
    let yahoo = serde_json::json!({});
    let quarters = parse_quarterly_financials("TEST", &yahoo);
    assert!(quarters.is_empty());
}

// ── parse_institutional_holders ────────────────────────────────

#[test]
fn parse_institutional_holders_basic() {
    let yahoo = serde_json::json!({
        "institutionOwnership": {
            "ownershipList": [
                {
                    "organization": "Vanguard Group",
                    "position": { "raw": 50_000_000 },
                    "pctHeld": { "raw": 0.08 },
                    "value": { "raw": 7_500_000_000.0 },
                    "reportDate": { "raw": 1711843200, "fmt": "2024-03-31" }
                },
                {
                    "organization": "BlackRock",
                    "position": { "raw": 40_000_000 },
                    "pctHeld": { "raw": 0.065 },
                    "value": { "raw": 6_000_000_000.0 },
                    "reportDate": { "raw": 1711843200, "fmt": "2024-03-31" }
                }
            ]
        }
    });

    let holders = parse_institutional_holders("TEST", &yahoo);
    assert_eq!(holders.len(), 2);
    assert_eq!(holders[0].holder_name, "Vanguard Group");
    assert_eq!(holders[0].shares, 50_000_000);
    assert_eq!(holders[1].holder_name, "BlackRock");
    assert_eq!(holders[1].symbol, "TEST");
}

#[test]
fn parse_institutional_holders_empty() {
    let yahoo = serde_json::json!({});
    let holders = parse_institutional_holders("TEST", &yahoo);
    assert!(holders.is_empty());
}

// ── SQLite CRUD roundtrip ──────────────────────────────────────

#[test]
fn upsert_and_get_fundamentals_roundtrip() {
    let conn = setup_test_db();
    let f = Fundamentals {
        symbol: "AAPL".to_string(),
        company_name: "Apple Inc".to_string(),
        sector: "Technology".to_string(),
        industry: "Consumer Electronics".to_string(),
        market_cap: Some(3_000_000_000_000.0),
        enterprise_value: Some(3_100_000_000_000.0),
        stock_price: Some(195.0),
        pe_ratio: Some(30.5),
        beta: Some(1.25),
        is_dividend_stock: true,
        dividend_yield: Some(0.005),
        last_updated: "2024-01-15T12:00:00Z".to_string(),
        ..Default::default()
    };
    upsert_fundamentals(&conn, &f).unwrap();

    let loaded = get_fundamentals(&conn, "AAPL").unwrap().unwrap();
    assert_eq!(loaded.symbol, "AAPL");
    assert_eq!(loaded.company_name, "Apple Inc");
    assert_eq!(loaded.market_cap, Some(3_000_000_000_000.0));
    assert!(loaded.is_dividend_stock);
    assert_eq!(loaded.pe_ratio, Some(30.5));
}

#[test]
fn upsert_fundamentals_updates_existing() {
    let conn = setup_test_db();
    let f1 = Fundamentals {
        symbol: "MSFT".to_string(),
        company_name: "Microsoft".to_string(),
        stock_price: Some(400.0),
        last_updated: "2024-01-01T00:00:00Z".to_string(),
        ..Default::default()
    };
    upsert_fundamentals(&conn, &f1).unwrap();

    let f2 = Fundamentals {
        symbol: "MSFT".to_string(),
        company_name: "Microsoft Corporation".to_string(),
        stock_price: Some(420.0),
        last_updated: "2024-02-01T00:00:00Z".to_string(),
        ..Default::default()
    };
    upsert_fundamentals(&conn, &f2).unwrap();

    let loaded = get_fundamentals(&conn, "MSFT").unwrap().unwrap();
    assert_eq!(loaded.company_name, "Microsoft Corporation");
    assert_eq!(loaded.stock_price, Some(420.0));
}

#[test]
fn get_fundamentals_not_found() {
    let conn = setup_test_db();
    let result = get_fundamentals(&conn, "ZZZZ").unwrap();
    assert!(result.is_none());
}

#[test]
fn fundamentals_scrape_order_targets_missing_then_oldest_then_recent() {
    let conn = setup_test_db();
    let fresh = Fundamentals {
        symbol: "WOK".to_string(),
        company_name: "WORK Medical".to_string(),
        last_updated: "2026-06-14T00:00:00Z".to_string(),
        ..Default::default()
    };
    let stale = Fundamentals {
        symbol: "AAPL".to_string(),
        company_name: "Apple".to_string(),
        last_updated: "2026-06-01T00:00:00Z".to_string(),
        ..Default::default()
    };
    upsert_fundamentals(&conn, &fresh).unwrap();
    upsert_fundamentals(&conn, &stale).unwrap();

    let mut tickers = vec!["WOK".to_string(), "AAPL".to_string(), "FNGR".to_string()];
    prioritize_fundamentals_symbols(&conn, &mut tickers, false);

    assert_eq!(tickers, vec!["FNGR", "AAPL", "WOK"]);
}

#[test]
fn get_all_fundamentals_multiple() {
    let conn = setup_test_db();
    for sym in ["AAPL", "GOOG", "MSFT"] {
        let f = Fundamentals {
            symbol: sym.to_string(),
            last_updated: "2024-01-01T00:00:00Z".to_string(),
            ..Default::default()
        };
        upsert_fundamentals(&conn, &f).unwrap();
    }
    let all = get_all_fundamentals(&conn).unwrap();
    assert_eq!(all.len(), 3);
    // Should be ordered alphabetically
    assert_eq!(all[0].symbol, "AAPL");
    assert_eq!(all[1].symbol, "GOOG");
    assert_eq!(all[2].symbol, "MSFT");
}

#[test]
fn upsert_and_get_quarterly_financials() {
    let conn = setup_test_db();
    let quarters = vec![
        QuarterlyFinancial {
            symbol: "TEST".to_string(),
            period_end: "2024-03-31".to_string(),
            total_revenue: Some(1_000_000.0),
            net_income: Some(100_000.0),
            free_cash_flow: Some(80_000.0),
            gross_profit: None,
            operating_income: None,
            ebitda: None,
            eps: Some(1.5),
        },
        QuarterlyFinancial {
            symbol: "TEST".to_string(),
            period_end: "2023-12-31".to_string(),
            total_revenue: Some(900_000.0),
            net_income: Some(90_000.0),
            free_cash_flow: None,
            gross_profit: None,
            operating_income: None,
            ebitda: None,
            eps: Some(1.3),
        },
    ];
    upsert_quarterly(&conn, &quarters).unwrap();

    let loaded = get_quarterly_financials(&conn, "TEST").unwrap();
    assert_eq!(loaded.len(), 2);
    // Ordered by period_end DESC
    assert_eq!(loaded[0].period_end, "2024-03-31");
    assert_eq!(loaded[0].total_revenue, Some(1_000_000.0));
    assert_eq!(loaded[1].period_end, "2023-12-31");
}

#[test]
fn upsert_and_get_institutional_holders() {
    let conn = setup_test_db();
    let holders = vec![InstitutionalHolder {
        symbol: "TEST".to_string(),
        holder_name: "Vanguard".to_string(),
        shares: 1_000_000,
        pct_held: 0.05,
        value: 150_000_000.0,
        date_reported: "2024-03-31".to_string(),
    }];
    upsert_holders(&conn, &holders).unwrap();

    let loaded = get_institutional_holders(&conn, "TEST").unwrap();
    assert_eq!(loaded.len(), 1);
    assert_eq!(loaded[0].holder_name, "Vanguard");
    assert_eq!(loaded[0].shares, 1_000_000);
}

// ── create_fundamentals_tables idempotent ──────────────────────

#[test]
fn create_tables_idempotent() {
    let conn = Connection::open_in_memory().unwrap();
    create_fundamentals_tables(&conn).unwrap();
    create_fundamentals_tables(&conn).unwrap(); // second call should not fail
}
