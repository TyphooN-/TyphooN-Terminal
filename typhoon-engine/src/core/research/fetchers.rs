use super::*;

// ── fetchers ───────────────────────────────────────────────────────

/// FMP /historical-price-full/stock_dividend/{symbol} — full dividend payment history.
pub async fn fetch_fmp_dividend_history(
    client: &reqwest::Client,
    symbol: &str,
    fmp_key: &str,
) -> Result<Vec<DividendRecord>, String> {
    if fmp_key.is_empty() {
        return Err("FMP API key required".into());
    }
    let url = format!(
        "https://financialmodelingprep.com/api/v3/historical-price-full/stock_dividend/{}?apikey={}",
        symbol, fmp_key
    );
    let resp = client
        .get(&url)
        .send()
        .await
        .map_err(|e| format!("FMP dividends failed: {e}"))?;
    if !resp.status().is_success() {
        return Err(format!("FMP dividends: HTTP {}", resp.status()));
    }
    let v: serde_json::Value = resp
        .json()
        .await
        .map_err(|e| format!("FMP dividends parse: {e}"))?;
    let mut rows = Vec::new();
    if let Some(arr) = v["historical"].as_array() {
        for e in arr {
            rows.push(DividendRecord {
                ex_date: e["date"].as_str().unwrap_or("").to_string(),
                pay_date: e["paymentDate"].as_str().unwrap_or("").to_string(),
                record_date: e["recordDate"].as_str().unwrap_or("").to_string(),
                declaration_date: e["declarationDate"].as_str().unwrap_or("").to_string(),
                amount: e["dividend"].as_f64().unwrap_or(0.0),
                adjusted_amount: e["adjDividend"].as_f64().unwrap_or(0.0),
                label: e["label"].as_str().unwrap_or("").to_string(),
            });
        }
    }
    Ok(rows)
}

/// FMP /analyst-estimates/{symbol} — forward EPS and revenue consensus estimates.
pub async fn fetch_fmp_earnings_estimates(
    client: &reqwest::Client,
    symbol: &str,
    fmp_key: &str,
) -> Result<Vec<EarningsEstimate>, String> {
    if fmp_key.is_empty() {
        return Err("FMP API key required".into());
    }
    let url = format!(
        "https://financialmodelingprep.com/api/v3/analyst-estimates/{}?apikey={}",
        symbol, fmp_key
    );
    let resp = client
        .get(&url)
        .send()
        .await
        .map_err(|e| format!("FMP estimates failed: {e}"))?;
    if !resp.status().is_success() {
        return Err(format!("FMP estimates: HTTP {}", resp.status()));
    }
    let arr: Vec<serde_json::Value> = resp
        .json()
        .await
        .map_err(|e| format!("FMP estimates parse: {e}"))?;
    let rows = arr
        .into_iter()
        .map(|e| EarningsEstimate {
            date: e["date"].as_str().unwrap_or("").to_string(),
            eps_avg: e["estimatedEpsAvg"].as_f64().unwrap_or(0.0),
            eps_high: e["estimatedEpsHigh"].as_f64().unwrap_or(0.0),
            eps_low: e["estimatedEpsLow"].as_f64().unwrap_or(0.0),
            revenue_avg: e["estimatedRevenueAvg"].as_f64().unwrap_or(0.0),
            revenue_high: e["estimatedRevenueHigh"].as_f64().unwrap_or(0.0),
            revenue_low: e["estimatedRevenueLow"].as_f64().unwrap_or(0.0),
            num_analysts_eps: e["numberAnalystEstimatedEps"].as_i64().unwrap_or(0) as i32,
            num_analysts_rev: e["numberAnalystsEstimatedRevenue"].as_i64().unwrap_or(0) as i32,
        })
        .collect();
    Ok(rows)
}

/// FMP /upgrades-downgrades (v4) — analyst rating change feed for a symbol.
pub async fn fetch_fmp_rating_changes(
    client: &reqwest::Client,
    symbol: &str,
    fmp_key: &str,
) -> Result<Vec<RatingChange>, String> {
    if fmp_key.is_empty() {
        return Err("FMP API key required".into());
    }
    let url = format!(
        "https://financialmodelingprep.com/api/v4/upgrades-downgrades?symbol={}&apikey={}",
        symbol, fmp_key
    );
    let resp = client
        .get(&url)
        .send()
        .await
        .map_err(|e| format!("FMP rating changes failed: {e}"))?;
    if !resp.status().is_success() {
        return Err(format!("FMP rating changes: HTTP {}", resp.status()));
    }
    let arr: Vec<serde_json::Value> = resp
        .json()
        .await
        .map_err(|e| format!("FMP rating changes parse: {e}"))?;
    let rows = arr
        .into_iter()
        .map(|e| {
            let to = e["newGrade"].as_str().unwrap_or("").to_string();
            let from = e["previousGrade"].as_str().unwrap_or("").to_string();
            let action_raw = e["action"].as_str().unwrap_or("").to_lowercase();
            // FMP action strings like "hold","buy" — map to upgrade/downgrade where we can.
            let action = if action_raw.is_empty() {
                if from.is_empty() {
                    "initiation"
                } else if to != from {
                    "changed"
                } else {
                    "maintain"
                }
                .to_string()
            } else {
                action_raw
            };
            RatingChange {
                date: e["publishedDate"]
                    .as_str()
                    .unwrap_or("")
                    .chars()
                    .take(10)
                    .collect(),
                symbol: e["symbol"].as_str().unwrap_or(symbol).to_uppercase(),
                company: e["gradingCompany"].as_str().unwrap_or("").to_string(),
                firm: e["gradingCompany"].as_str().unwrap_or("").to_string(),
                action,
                from_grade: from,
                to_grade: to,
                price_target: e["priceTarget"].as_f64().unwrap_or(0.0),
            }
        })
        .collect();
    Ok(rows)
}

/// Yahoo batch quote → Treasury yield curve snapshot (no auth).
pub async fn fetch_treasury_yields(client: &reqwest::Client) -> Result<Vec<TreasuryYield>, String> {
    let tickers: Vec<&str> = TREASURY_TENORS.iter().map(|(t, _)| *t).collect();
    let quotes = fetch_yahoo_quotes(client, &tickers).await?;
    let mut out = Vec::new();
    for (sym, price, change, pct) in quotes {
        if let Some((_, tenor)) = TREASURY_TENORS.iter().find(|(t, _)| *t == sym.as_str()) {
            out.push(TreasuryYield {
                tenor: (*tenor).to_string(),
                ticker: sym,
                yield_pct: price,
                change,
                change_pct: pct,
            });
        }
    }
    // Preserve ladder order (13W, 5Y, 10Y, 30Y).
    out.sort_by_key(|t| {
        TREASURY_TENORS
            .iter()
            .position(|(_, lbl)| *lbl == t.tenor.as_str())
            .unwrap_or(99)
    });
    Ok(out)
}

// ── fetchers ───────────────────────────────────────────────────────

/// Parse a Socrata numeric field that arrives as either a JSON number or a string.
fn socrata_f64(v: &serde_json::Value) -> f64 {
    v.as_f64()
        .or_else(|| v.as_str().and_then(|s| s.parse().ok()))
        .unwrap_or(0.0)
}

/// FMP /income-statement/{symbol} — up to 20 historical periods. `period` = "annual" or "quarter".
pub async fn fetch_fmp_income_statement(
    client: &reqwest::Client,
    symbol: &str,
    period: &str,
    fmp_key: &str,
) -> Result<Vec<IncomeStatement>, String> {
    if fmp_key.is_empty() {
        return Err("FMP API key required".into());
    }
    let url = format!(
        "https://financialmodelingprep.com/api/v3/income-statement/{}?period={}&limit=20&apikey={}",
        symbol, period, fmp_key
    );
    let resp = client
        .get(&url)
        .send()
        .await
        .map_err(|e| format!("FMP income failed: {e}"))?;
    if !resp.status().is_success() {
        return Err(format!("FMP income: HTTP {}", resp.status()));
    }
    let arr: Vec<serde_json::Value> = resp
        .json()
        .await
        .map_err(|e| format!("FMP income parse: {e}"))?;
    let rows = arr
        .into_iter()
        .map(|e| IncomeStatement {
            date: e["date"].as_str().unwrap_or("").to_string(),
            period: e["period"].as_str().unwrap_or("").to_string(),
            revenue: e["revenue"].as_f64().unwrap_or(0.0),
            cost_of_revenue: e["costOfRevenue"].as_f64().unwrap_or(0.0),
            gross_profit: e["grossProfit"].as_f64().unwrap_or(0.0),
            research_and_development: e["researchAndDevelopmentExpenses"].as_f64().unwrap_or(0.0),
            selling_general_admin: e["sellingGeneralAndAdministrativeExpenses"]
                .as_f64()
                .unwrap_or(0.0),
            operating_expenses: e["operatingExpenses"].as_f64().unwrap_or(0.0),
            operating_income: e["operatingIncome"].as_f64().unwrap_or(0.0),
            interest_expense: e["interestExpense"].as_f64().unwrap_or(0.0),
            ebitda: e["ebitda"].as_f64().unwrap_or(0.0),
            income_before_tax: e["incomeBeforeTax"].as_f64().unwrap_or(0.0),
            income_tax_expense: e["incomeTaxExpense"].as_f64().unwrap_or(0.0),
            net_income: e["netIncome"].as_f64().unwrap_or(0.0),
            eps: e["eps"].as_f64().unwrap_or(0.0),
            eps_diluted: e["epsdiluted"].as_f64().unwrap_or(0.0),
            weighted_shares_out: e["weightedAverageShsOut"].as_f64().unwrap_or(0.0),
        })
        .collect();
    Ok(rows)
}

/// FMP /balance-sheet-statement/{symbol} — up to 20 historical periods.
pub async fn fetch_fmp_balance_sheet(
    client: &reqwest::Client,
    symbol: &str,
    period: &str,
    fmp_key: &str,
) -> Result<Vec<BalanceSheet>, String> {
    if fmp_key.is_empty() {
        return Err("FMP API key required".into());
    }
    let url = format!(
        "https://financialmodelingprep.com/api/v3/balance-sheet-statement/{}?period={}&limit=20&apikey={}",
        symbol, period, fmp_key
    );
    let resp = client
        .get(&url)
        .send()
        .await
        .map_err(|e| format!("FMP balance failed: {e}"))?;
    if !resp.status().is_success() {
        return Err(format!("FMP balance: HTTP {}", resp.status()));
    }
    let arr: Vec<serde_json::Value> = resp
        .json()
        .await
        .map_err(|e| format!("FMP balance parse: {e}"))?;
    let rows = arr
        .into_iter()
        .map(|e| BalanceSheet {
            date: e["date"].as_str().unwrap_or("").to_string(),
            period: e["period"].as_str().unwrap_or("").to_string(),
            cash_and_equiv: e["cashAndCashEquivalents"].as_f64().unwrap_or(0.0),
            short_term_investments: e["shortTermInvestments"].as_f64().unwrap_or(0.0),
            net_receivables: e["netReceivables"].as_f64().unwrap_or(0.0),
            inventory: e["inventory"].as_f64().unwrap_or(0.0),
            total_current_assets: e["totalCurrentAssets"].as_f64().unwrap_or(0.0),
            property_plant_equipment: e["propertyPlantEquipmentNet"].as_f64().unwrap_or(0.0),
            goodwill: e["goodwill"].as_f64().unwrap_or(0.0),
            intangible_assets: e["intangibleAssets"].as_f64().unwrap_or(0.0),
            long_term_investments: e["longTermInvestments"].as_f64().unwrap_or(0.0),
            total_non_current_assets: e["totalNonCurrentAssets"].as_f64().unwrap_or(0.0),
            total_assets: e["totalAssets"].as_f64().unwrap_or(0.0),
            accounts_payable: e["accountPayables"].as_f64().unwrap_or(0.0),
            short_term_debt: e["shortTermDebt"].as_f64().unwrap_or(0.0),
            total_current_liabilities: e["totalCurrentLiabilities"].as_f64().unwrap_or(0.0),
            long_term_debt: e["longTermDebt"].as_f64().unwrap_or(0.0),
            total_non_current_liabilities: e["totalNonCurrentLiabilities"].as_f64().unwrap_or(0.0),
            total_liabilities: e["totalLiabilities"].as_f64().unwrap_or(0.0),
            common_stock: e["commonStock"].as_f64().unwrap_or(0.0),
            retained_earnings: e["retainedEarnings"].as_f64().unwrap_or(0.0),
            total_equity: e["totalStockholdersEquity"].as_f64().unwrap_or(0.0),
            total_debt: e["totalDebt"].as_f64().unwrap_or(0.0),
            net_debt: e["netDebt"].as_f64().unwrap_or(0.0),
        })
        .collect();
    Ok(rows)
}

/// FMP /cash-flow-statement/{symbol} — up to 20 historical periods.
pub async fn fetch_fmp_cash_flow(
    client: &reqwest::Client,
    symbol: &str,
    period: &str,
    fmp_key: &str,
) -> Result<Vec<CashFlowStatement>, String> {
    if fmp_key.is_empty() {
        return Err("FMP API key required".into());
    }
    let url = format!(
        "https://financialmodelingprep.com/api/v3/cash-flow-statement/{}?period={}&limit=20&apikey={}",
        symbol, period, fmp_key
    );
    let resp = client
        .get(&url)
        .send()
        .await
        .map_err(|e| format!("FMP cash flow failed: {e}"))?;
    if !resp.status().is_success() {
        return Err(format!("FMP cash flow: HTTP {}", resp.status()));
    }
    let arr: Vec<serde_json::Value> = resp
        .json()
        .await
        .map_err(|e| format!("FMP cash flow parse: {e}"))?;
    let rows = arr
        .into_iter()
        .map(|e| CashFlowStatement {
            date: e["date"].as_str().unwrap_or("").to_string(),
            period: e["period"].as_str().unwrap_or("").to_string(),
            net_income: e["netIncome"].as_f64().unwrap_or(0.0),
            depreciation_amortization: e["depreciationAndAmortization"].as_f64().unwrap_or(0.0),
            stock_based_comp: e["stockBasedCompensation"].as_f64().unwrap_or(0.0),
            change_working_capital: e["changeInWorkingCapital"].as_f64().unwrap_or(0.0),
            cash_from_operations: e["operatingCashFlow"].as_f64().unwrap_or(0.0),
            capex: e["capitalExpenditure"].as_f64().unwrap_or(0.0),
            acquisitions: e["acquisitionsNet"].as_f64().unwrap_or(0.0),
            investments_purchases: e["purchasesOfInvestments"].as_f64().unwrap_or(0.0),
            cash_from_investing: e["netCashUsedForInvestingActivites"]
                .as_f64()
                .unwrap_or(0.0),
            debt_repayment: e["debtRepayment"].as_f64().unwrap_or(0.0),
            dividends_paid: e["dividendsPaid"].as_f64().unwrap_or(0.0),
            stock_repurchases: e["commonStockRepurchased"].as_f64().unwrap_or(0.0),
            cash_from_financing: e["netCashUsedProvidedByFinancingActivities"]
                .as_f64()
                .unwrap_or(0.0),
            net_change_cash: e["netChangeInCash"].as_f64().unwrap_or(0.0),
            free_cash_flow: e["freeCashFlow"].as_f64().unwrap_or(0.0),
        })
        .collect();
    Ok(rows)
}

/// Convenience: fetch the full FA bundle (all 3 statements × annual+quarterly) in one call.
/// 6 FMP calls, 400 ms between each = ~2.4 s per symbol.
pub async fn fetch_fmp_financial_bundle(
    client: &reqwest::Client,
    symbol: &str,
    fmp_key: &str,
) -> Result<FinancialStatements, String> {
    let mut bundle = FinancialStatements::default();
    bundle.income_annual = fetch_fmp_income_statement(client, symbol, "annual", fmp_key)
        .await
        .unwrap_or_default();
    tokio::time::sleep(std::time::Duration::from_millis(400)).await;
    bundle.income_quarterly = fetch_fmp_income_statement(client, symbol, "quarter", fmp_key)
        .await
        .unwrap_or_default();
    tokio::time::sleep(std::time::Duration::from_millis(400)).await;
    bundle.balance_annual = fetch_fmp_balance_sheet(client, symbol, "annual", fmp_key)
        .await
        .unwrap_or_default();
    tokio::time::sleep(std::time::Duration::from_millis(400)).await;
    bundle.balance_quarterly = fetch_fmp_balance_sheet(client, symbol, "quarter", fmp_key)
        .await
        .unwrap_or_default();
    tokio::time::sleep(std::time::Duration::from_millis(400)).await;
    bundle.cashflow_annual = fetch_fmp_cash_flow(client, symbol, "annual", fmp_key)
        .await
        .unwrap_or_default();
    tokio::time::sleep(std::time::Duration::from_millis(400)).await;
    bundle.cashflow_quarterly = fetch_fmp_cash_flow(client, symbol, "quarter", fmp_key)
        .await
        .unwrap_or_default();
    Ok(bundle)
}

/// Finnhub /stock/executive — company officers with compensation.
pub async fn fetch_finnhub_executives(
    client: &reqwest::Client,
    symbol: &str,
    token: &str,
) -> Result<Vec<Executive>, String> {
    if token.is_empty() {
        return Err("Finnhub API key required".into());
    }
    let resp = client
        .get("https://finnhub.io/api/v1/stock/executive")
        .query(&[("symbol", symbol), ("token", token)])
        .send()
        .await
        .map_err(|e| format!("Finnhub executives failed: {e}"))?;
    if !resp.status().is_success() {
        return Err(format!("Finnhub executives: HTTP {}", resp.status()));
    }
    let v: serde_json::Value = resp
        .json()
        .await
        .map_err(|e| format!("Finnhub executives parse: {e}"))?;
    let mut rows = Vec::new();
    if let Some(arr) = v["executive"].as_array() {
        for e in arr {
            rows.push(Executive {
                name: e["name"].as_str().unwrap_or("").to_string(),
                position: e["position"].as_str().unwrap_or("").to_string(),
                age: e["age"].as_i64().unwrap_or(0) as i32,
                sex: e["sex"].as_str().unwrap_or("").to_string(),
                since: e["since"].as_str().unwrap_or("").to_string(),
                compensation: e["compensation"].as_f64().unwrap_or(0.0),
                year: e["year"].as_i64().unwrap_or(0) as i32,
            });
        }
    }
    Ok(rows)
}

/// CFTC Socrata — Commitments of Traders, Legacy Futures combined.
/// Public JSON endpoint, no API key. Returns one row per market for the most recent report date.
/// WoW change in non-commercial net is computed from the prior week found in the same payload.
pub async fn fetch_cftc_cot(client: &reqwest::Client) -> Result<Vec<CotReport>, String> {
    // Legacy futures-only combined. Ordered by report date descending so the first rows
    // define the latest week, subsequent rows include the prior week for WoW delta.
    let url = "https://publicreporting.cftc.gov/resource/6dca-aqww.json?\
               $limit=2000&$order=report_date_as_yyyy_mm_dd DESC";
    let resp = client
        .get(url)
        .header(
            "User-Agent",
            "Mozilla/5.0 (X11; Linux x86_64) TyphooN-Terminal/0.1",
        )
        .send()
        .await
        .map_err(|e| format!("CFTC COT failed: {e}"))?;
    if !resp.status().is_success() {
        return Err(format!("CFTC COT: HTTP {}", resp.status()));
    }
    let arr: Vec<serde_json::Value> = resp
        .json()
        .await
        .map_err(|e| format!("CFTC COT parse: {e}"))?;
    if arr.is_empty() {
        return Ok(vec![]);
    }

    // Latest report date is the max date seen in the payload (rows come sorted DESC but be safe).
    let latest_date = arr
        .iter()
        .filter_map(|e| e["report_date_as_yyyy_mm_dd"].as_str())
        .map(|s| s.chars().take(10).collect::<String>())
        .max()
        .unwrap_or_default();
    if latest_date.is_empty() {
        return Ok(vec![]);
    }

    // For each market, remember the first (latest) non-commercial net and the first *prior-week* net.
    use std::collections::HashMap;
    let mut prior: HashMap<String, f64> = HashMap::new();
    for e in arr.iter() {
        let market = e["market_and_exchange_names"]
            .as_str()
            .unwrap_or("")
            .to_string();
        if market.is_empty() {
            continue;
        }
        let date: String = e["report_date_as_yyyy_mm_dd"]
            .as_str()
            .unwrap_or("")
            .chars()
            .take(10)
            .collect();
        if date == latest_date {
            continue;
        }
        let nc_net = socrata_f64(&e["noncomm_positions_long_all"])
            - socrata_f64(&e["noncomm_positions_short_all"]);
        prior.entry(market).or_insert(nc_net);
    }

    // Build the latest-week rows.
    let mut rows = Vec::new();
    for e in arr.iter() {
        let date: String = e["report_date_as_yyyy_mm_dd"]
            .as_str()
            .unwrap_or("")
            .chars()
            .take(10)
            .collect();
        if date != latest_date {
            continue;
        }
        let market = e["market_and_exchange_names"]
            .as_str()
            .unwrap_or("")
            .to_string();
        if market.is_empty() {
            continue;
        }
        let nc_long = socrata_f64(&e["noncomm_positions_long_all"]);
        let nc_short = socrata_f64(&e["noncomm_positions_short_all"]);
        let net = nc_long - nc_short;
        let prev = prior.get(&market).copied().unwrap_or(net);
        rows.push(CotReport {
            market_name: market,
            market_code: e["cftc_contract_market_code"]
                .as_str()
                .unwrap_or("")
                .to_string(),
            report_date: date,
            open_interest: socrata_f64(&e["open_interest_all"]),
            noncomm_long: nc_long,
            noncomm_short: nc_short,
            // Socrata column name intentionally has the typo from the CFTC source feed.
            noncomm_spreads: socrata_f64(&e["noncomm_postions_spread_all"]),
            comm_long: socrata_f64(&e["comm_positions_long_all"]),
            comm_short: socrata_f64(&e["comm_positions_short_all"]),
            nonrept_long: socrata_f64(&e["nonrept_positions_long_all"]),
            nonrept_short: socrata_f64(&e["nonrept_positions_short_all"]),
            noncomm_net: net,
            noncomm_net_change: net - prev,
        });
    }
    rows.sort_by(|a, b| a.market_name.cmp(&b.market_name));
    Ok(rows)
}

// ── fetchers ───────────────────────────────────────────────────────

/// FMP /historical-price-full/stock_split/{symbol} — historical stock splits.
pub async fn fetch_fmp_stock_splits(
    client: &reqwest::Client,
    symbol: &str,
    fmp_key: &str,
) -> Result<Vec<StockSplit>, String> {
    if fmp_key.is_empty() {
        return Err("FMP API key required".into());
    }
    let url = format!(
        "https://financialmodelingprep.com/api/v3/historical-price-full/stock_split/{}?apikey={}",
        symbol, fmp_key
    );
    let resp = client
        .get(&url)
        .send()
        .await
        .map_err(|e| format!("FMP splits failed: {e}"))?;
    if !resp.status().is_success() {
        return Err(format!("FMP splits: HTTP {}", resp.status()));
    }
    let v: serde_json::Value = resp
        .json()
        .await
        .map_err(|e| format!("FMP splits parse: {e}"))?;
    let mut rows = Vec::new();
    if let Some(arr) = v["historical"].as_array() {
        for e in arr {
            let num = e["numerator"].as_f64().unwrap_or(0.0);
            let den = e["denominator"].as_f64().unwrap_or(0.0);
            let label = e["label"]
                .as_str()
                .map(|s| s.to_string())
                .unwrap_or_else(|| {
                    if num > 0.0 && den > 0.0 {
                        format!("{}:{}", num, den)
                    } else {
                        String::new()
                    }
                });
            rows.push(StockSplit {
                date: e["date"].as_str().unwrap_or("").to_string(),
                label,
                numerator: num,
                denominator: den,
            });
        }
    }
    Ok(rows)
}

/// Yahoo chart split events — public fallback for stock splits/reverse splits.
///
/// FMP's free split feed misses some microcap actions; Yahoo's chart endpoint is
/// the same public source used by its finance UI and often has the split event
/// immediately. This path needs no API key and gives the chart merge enough
/// corporate-action data to invalidate/rebuild stale adjusted bars.
pub async fn fetch_yahoo_stock_splits(
    client: &reqwest::Client,
    symbol: &str,
) -> Result<Vec<StockSplit>, String> {
    let symbol = symbol.trim().trim_end_matches(".EQ").to_ascii_uppercase();
    if symbol.is_empty() {
        return Ok(Vec::new());
    }
    let resp = client
        .get(format!(
            "https://query1.finance.yahoo.com/v8/finance/chart/{symbol}"
        ))
        .query(&[
            ("period1", "0"),
            ("period2", "4102444800"),
            ("interval", "1d"),
            ("events", "split"),
        ])
        .send()
        .await
        .map_err(|e| format!("Yahoo splits failed: {e}"))?;
    if !resp.status().is_success() {
        return Err(format!("Yahoo splits: HTTP {}", resp.status()));
    }
    let v: serde_json::Value = resp
        .json()
        .await
        .map_err(|e| format!("Yahoo splits parse: {e}"))?;
    parse_yahoo_stock_splits_value(&v)
}

/// Fetch stock splits from all available sources. FMP is used when a key exists;
/// Yahoo is always tried as a no-key fallback/supplement, because fresh reverse
/// splits can be missing from FMP but present in Yahoo chart events.
pub async fn fetch_stock_splits(
    client: &reqwest::Client,
    symbol: &str,
    fmp_key: &str,
) -> Result<Vec<StockSplit>, String> {
    let mut rows = Vec::new();
    let mut errors = Vec::new();

    if !fmp_key.is_empty() {
        match fetch_fmp_stock_splits(client, symbol, fmp_key).await {
            Ok(mut fmp_rows) => rows.append(&mut fmp_rows),
            Err(e) => errors.push(e),
        }
    }

    match fetch_yahoo_stock_splits(client, symbol).await {
        Ok(yahoo_rows) => {
            let split_key = |date: &str, numerator: f64, denominator: f64| {
                (
                    date.to_string(),
                    (numerator * 1_000_000_000.0).round() as i64,
                    (denominator * 1_000_000_000.0).round() as i64,
                )
            };
            let mut seen: std::collections::HashSet<(String, i64, i64)> = rows
                .iter()
                .map(|old: &StockSplit| split_key(&old.date, old.numerator, old.denominator))
                .collect();
            for split in yahoo_rows {
                if seen.insert(split_key(&split.date, split.numerator, split.denominator)) {
                    rows.push(split);
                }
            }
        }
        Err(e) => errors.push(e),
    }

    if rows.is_empty() && !errors.is_empty() {
        return Err(errors.join("; "));
    }
    rows.sort_by(|a, b| b.date.cmp(&a.date));
    Ok(rows)
}

pub(crate) fn parse_yahoo_stock_splits_value(
    v: &serde_json::Value,
) -> Result<Vec<StockSplit>, String> {
    let Some(result) = v["chart"]["result"].as_array().and_then(|arr| arr.first()) else {
        return Ok(Vec::new());
    };
    let Some(splits) = result["events"]["splits"].as_object() else {
        return Ok(Vec::new());
    };
    let mut rows = Vec::new();
    for event in splits.values() {
        let ts = event["date"].as_i64().unwrap_or(0);
        let date = chrono::DateTime::from_timestamp(ts, 0)
            .map(|dt| dt.date_naive().to_string())
            .unwrap_or_default();
        if date.is_empty() {
            continue;
        }
        let (mut numerator, mut denominator) = (
            event["numerator"].as_f64().unwrap_or(0.0),
            event["denominator"].as_f64().unwrap_or(0.0),
        );
        if (numerator <= 0.0 || denominator <= 0.0)
            && let Some((n, d)) = parse_split_ratio(event["splitRatio"].as_str().unwrap_or(""))
        {
            numerator = n;
            denominator = d;
        }
        if numerator <= 0.0 || denominator <= 0.0 {
            continue;
        }
        rows.push(StockSplit {
            date,
            label: format!("{}:{}", numerator, denominator),
            numerator,
            denominator,
        });
    }
    rows.sort_by(|a, b| b.date.cmp(&a.date));
    Ok(rows)
}

fn parse_split_ratio(raw: &str) -> Option<(f64, f64)> {
    let (left, right) = raw.split_once(':').or_else(|| raw.split_once('/'))?;
    let numerator = left.trim().parse::<f64>().ok()?;
    let denominator = right.trim().parse::<f64>().ok()?;
    (numerator > 0.0 && denominator > 0.0).then_some((numerator, denominator))
}

/// FMP /etf-holder/{symbol} — up to 1000 constituent holdings of an ETF.
pub async fn fetch_fmp_etf_holdings(
    client: &reqwest::Client,
    symbol: &str,
    fmp_key: &str,
) -> Result<Vec<EtfHolding>, String> {
    if fmp_key.is_empty() {
        return Err("FMP API key required".into());
    }
    let url = format!(
        "https://financialmodelingprep.com/api/v3/etf-holder/{}?apikey={}",
        symbol, fmp_key
    );
    let resp = client
        .get(&url)
        .send()
        .await
        .map_err(|e| format!("FMP etf-holder failed: {e}"))?;
    if !resp.status().is_success() {
        return Err(format!("FMP etf-holder: HTTP {}", resp.status()));
    }
    let arr: Vec<serde_json::Value> = resp
        .json()
        .await
        .map_err(|e| format!("FMP etf-holder parse: {e}"))?;
    let rows = arr
        .into_iter()
        .map(|e| EtfHolding {
            symbol: e["asset"].as_str().unwrap_or("").to_string(),
            name: e["name"].as_str().unwrap_or("").to_string(),
            weight_pct: e["weightPercentage"].as_f64().unwrap_or(0.0),
            shares: e["sharesNumber"].as_f64().unwrap_or(0.0),
            market_value: e["marketValue"].as_f64().unwrap_or(0.0),
            updated: e["updated"].as_str().unwrap_or("").to_string(),
        })
        .collect();
    Ok(rows)
}

/// Finnhub /stock/recommendation — last ~12 months of monthly buy/hold/sell bucket counts.
pub async fn fetch_finnhub_recommendations(
    client: &reqwest::Client,
    symbol: &str,
    token: &str,
) -> Result<Vec<AnalystRecommendation>, String> {
    if token.is_empty() {
        return Err("Finnhub API key required".into());
    }
    let resp = client
        .get("https://finnhub.io/api/v1/stock/recommendation")
        .query(&[("symbol", symbol), ("token", token)])
        .send()
        .await
        .map_err(|e| format!("Finnhub recommendations failed: {e}"))?;
    if !resp.status().is_success() {
        return Err(format!("Finnhub recommendations: HTTP {}", resp.status()));
    }
    let arr: Vec<serde_json::Value> = resp
        .json()
        .await
        .map_err(|e| format!("Finnhub recommendations parse: {e}"))?;
    let rows = arr
        .into_iter()
        .map(|e| AnalystRecommendation {
            period: e["period"].as_str().unwrap_or("").to_string(),
            strong_buy: e["strongBuy"].as_i64().unwrap_or(0) as i32,
            buy: e["buy"].as_i64().unwrap_or(0) as i32,
            hold: e["hold"].as_i64().unwrap_or(0) as i32,
            sell: e["sell"].as_i64().unwrap_or(0) as i32,
            strong_sell: e["strongSell"].as_i64().unwrap_or(0) as i32,
        })
        .collect();
    Ok(rows)
}

/// Finnhub /stock/price-target — consensus high/low/mean target snapshot.
pub async fn fetch_finnhub_price_target(
    client: &reqwest::Client,
    symbol: &str,
    token: &str,
) -> Result<PriceTarget, String> {
    if token.is_empty() {
        return Err("Finnhub API key required".into());
    }
    let resp = client
        .get("https://finnhub.io/api/v1/stock/price-target")
        .query(&[("symbol", symbol), ("token", token)])
        .send()
        .await
        .map_err(|e| format!("Finnhub price-target failed: {e}"))?;
    if !resp.status().is_success() {
        return Err(format!("Finnhub price-target: HTTP {}", resp.status()));
    }
    let v: serde_json::Value = resp
        .json()
        .await
        .map_err(|e| format!("Finnhub price-target parse: {e}"))?;
    Ok(PriceTarget {
        symbol: symbol.to_uppercase(),
        target_high: v["targetHigh"].as_f64().unwrap_or(0.0),
        target_low: v["targetLow"].as_f64().unwrap_or(0.0),
        target_mean: v["targetMean"].as_f64().unwrap_or(0.0),
        target_median: v["targetMedian"].as_f64().unwrap_or(0.0),
        last_updated: v["lastUpdated"]
            .as_str()
            .unwrap_or("")
            .chars()
            .take(10)
            .collect(),
        num_analysts: v["numberOfAnalysts"].as_i64().unwrap_or(0) as i32,
    })
}

/// FMP /esg-environmental-social-governance-data?symbol={sym} — historical ESG score rows.
pub async fn fetch_fmp_esg(
    client: &reqwest::Client,
    symbol: &str,
    fmp_key: &str,
) -> Result<Vec<EsgScore>, String> {
    if fmp_key.is_empty() {
        return Err("FMP API key required".into());
    }
    let url = format!(
        "https://financialmodelingprep.com/api/v4/esg-environmental-social-governance-data?symbol={}&apikey={}",
        symbol, fmp_key
    );
    let resp = client
        .get(&url)
        .send()
        .await
        .map_err(|e| format!("FMP esg failed: {e}"))?;
    if !resp.status().is_success() {
        return Err(format!("FMP esg: HTTP {}", resp.status()));
    }
    let arr: Vec<serde_json::Value> = resp
        .json()
        .await
        .map_err(|e| format!("FMP esg parse: {e}"))?;
    let rows = arr
        .into_iter()
        .map(|e| EsgScore {
            symbol: e["symbol"].as_str().unwrap_or(symbol).to_uppercase(),
            environmental_score: e["environmentalScore"].as_f64().unwrap_or(0.0),
            social_score: e["socialScore"].as_f64().unwrap_or(0.0),
            governance_score: e["governanceScore"].as_f64().unwrap_or(0.0),
            esg_score: e["ESGScore"].as_f64().unwrap_or(0.0),
            year: e["year"].as_i64().unwrap_or(0) as i32,
        })
        .collect();
    Ok(rows)
}

/// FMP index constituent endpoint (/sp500_constituent, /nasdaq_constituent, /dowjones_constituent).
/// `index_code` accepts "SP500" | "NDX" | "DJIA"; mapped to the right FMP path.
pub async fn fetch_fmp_index_members(
    client: &reqwest::Client,
    index_code: &str,
    fmp_key: &str,
) -> Result<Vec<IndexMember>, String> {
    if fmp_key.is_empty() {
        return Err("FMP API key required".into());
    }
    let (path, idx_label) = match index_code.to_uppercase().as_str() {
        "SP500" | "SPX" | "S&P500" => ("sp500_constituent", "SP500"),
        "NDX" | "NASDAQ" | "NDX100" => ("nasdaq_constituent", "NDX"),
        "DJIA" | "DOW" | "INDU" => ("dowjones_constituent", "DJIA"),
        other => return Err(format!("Unknown index code: {}", other)),
    };
    let url = format!(
        "https://financialmodelingprep.com/api/v3/{}?apikey={}",
        path, fmp_key
    );
    let resp = client
        .get(&url)
        .send()
        .await
        .map_err(|e| format!("FMP index members failed: {e}"))?;
    if !resp.status().is_success() {
        return Err(format!("FMP index members: HTTP {}", resp.status()));
    }
    let arr: Vec<serde_json::Value> = resp
        .json()
        .await
        .map_err(|e| format!("FMP index members parse: {e}"))?;
    let rows = arr
        .into_iter()
        .map(|e| IndexMember {
            index: idx_label.to_string(),
            symbol: e["symbol"].as_str().unwrap_or("").to_uppercase(),
            name: e["name"].as_str().unwrap_or("").to_string(),
            sector: e["sector"].as_str().unwrap_or("").to_string(),
            sub_sector: e["subSector"].as_str().unwrap_or("").to_string(),
            headquarters: e["headQuarter"].as_str().unwrap_or("").to_string(),
            date_added: e["dateFirstAdded"].as_str().unwrap_or("").to_string(),
        })
        .collect();
    Ok(rows)
}

// ── fetchers ──

/// FMP /v4/insider-trading — SEC Form 4 insider trade rows (default page=0, up to 100 rows).
pub async fn fetch_fmp_insider_trades(
    client: &reqwest::Client,
    symbol: &str,
    fmp_key: &str,
) -> Result<Vec<InsiderTrade>, String> {
    if fmp_key.is_empty() {
        return Err("FMP API key required".into());
    }
    let url = format!(
        "https://financialmodelingprep.com/api/v4/insider-trading?symbol={}&page=0&apikey={}",
        symbol, fmp_key
    );
    let resp = client
        .get(&url)
        .send()
        .await
        .map_err(|e| format!("FMP insider failed: {e}"))?;
    if !resp.status().is_success() {
        return Err(format!("FMP insider: HTTP {}", resp.status()));
    }
    let arr: Vec<serde_json::Value> = resp
        .json()
        .await
        .map_err(|e| format!("FMP insider parse: {e}"))?;
    let rows = arr
        .into_iter()
        .map(|e| {
            let shares = e["securitiesTransacted"].as_f64().unwrap_or(0.0);
            let price = e["price"].as_f64().unwrap_or(0.0);
            InsiderTrade {
                filing_date: e["filingDate"]
                    .as_str()
                    .unwrap_or("")
                    .chars()
                    .take(10)
                    .collect(),
                transaction_date: e["transactionDate"]
                    .as_str()
                    .unwrap_or("")
                    .chars()
                    .take(10)
                    .collect(),
                reporting_name: e["reportingName"].as_str().unwrap_or("").to_string(),
                transaction_type: e["transactionType"].as_str().unwrap_or("").to_string(),
                acquisition_disposition: e["acquistionOrDisposition"]
                    .as_str()
                    .unwrap_or("")
                    .to_string(),
                shares,
                price,
                value_usd: shares * price,
                shares_owned_after: e["securitiesOwned"].as_f64().unwrap_or(0.0),
                link: e["link"].as_str().unwrap_or("").to_string(),
            }
        })
        .collect();
    Ok(rows)
}

/// FMP /v3/institutional-holder/{symbol} — 13F-derived top holders of a stock.
pub async fn fetch_fmp_institutional_holders(
    client: &reqwest::Client,
    symbol: &str,
    fmp_key: &str,
) -> Result<Vec<InstitutionalHolder>, String> {
    if fmp_key.is_empty() {
        return Err("FMP API key required".into());
    }
    let url = format!(
        "https://financialmodelingprep.com/api/v3/institutional-holder/{}?apikey={}",
        symbol, fmp_key
    );
    let resp = client
        .get(&url)
        .send()
        .await
        .map_err(|e| format!("FMP holders failed: {e}"))?;
    if !resp.status().is_success() {
        return Err(format!("FMP holders: HTTP {}", resp.status()));
    }
    let arr: Vec<serde_json::Value> = resp
        .json()
        .await
        .map_err(|e| format!("FMP holders parse: {e}"))?;
    let rows = arr
        .into_iter()
        .map(|e| InstitutionalHolder {
            holder: e["holder"].as_str().unwrap_or("").to_string(),
            shares: e["shares"].as_f64().unwrap_or(0.0),
            date_reported: e["dateReported"]
                .as_str()
                .unwrap_or("")
                .chars()
                .take(10)
                .collect(),
            change: e["change"].as_f64().unwrap_or(0.0),
        })
        .collect();
    Ok(rows)
}

/// FMP /v4/shares_float?symbol=… — latest free-float / outstanding-shares snapshot.
pub async fn fetch_fmp_shares_float(
    client: &reqwest::Client,
    symbol: &str,
    fmp_key: &str,
) -> Result<SharesFloat, String> {
    if fmp_key.is_empty() {
        return Err("FMP API key required".into());
    }
    let url = format!(
        "https://financialmodelingprep.com/api/v4/shares_float?symbol={}&apikey={}",
        symbol, fmp_key
    );
    let resp = client
        .get(&url)
        .send()
        .await
        .map_err(|e| format!("FMP shares_float failed: {e}"))?;
    if !resp.status().is_success() {
        return Err(format!("FMP shares_float: HTTP {}", resp.status()));
    }
    let v: serde_json::Value = resp
        .json()
        .await
        .map_err(|e| format!("FMP shares_float parse: {e}"))?;
    // Response is a 1-element array or a bare object — handle both.
    let e = if let Some(first) = v.as_array().and_then(|a| a.first()) {
        first.clone()
    } else {
        v
    };
    Ok(SharesFloat {
        symbol: e["symbol"].as_str().unwrap_or(symbol).to_uppercase(),
        date: e["date"].as_str().unwrap_or("").chars().take(10).collect(),
        free_float_pct: e["freeFloat"].as_f64().unwrap_or(0.0),
        float_shares: e["floatShares"].as_f64().unwrap_or(0.0),
        outstanding_shares: e["outstandingShares"].as_f64().unwrap_or(0.0),
        source: e["source"].as_str().unwrap_or("").to_string(),
    })
}

/// FMP /v3/historical-price-full/{symbol} — up to ~5 years of daily OHLCV.
/// `limit` is applied client-side after parsing (FMP returns all history by default).
pub async fn fetch_fmp_historical_price(
    client: &reqwest::Client,
    symbol: &str,
    fmp_key: &str,
    limit: usize,
) -> Result<Vec<HistoricalPriceRow>, String> {
    if fmp_key.is_empty() {
        return Err("FMP API key required".into());
    }
    let url = format!(
        "https://financialmodelingprep.com/api/v3/historical-price-full/{}?apikey={}",
        symbol, fmp_key
    );
    let resp = client
        .get(&url)
        .send()
        .await
        .map_err(|e| format!("FMP historical failed: {e}"))?;
    if !resp.status().is_success() {
        return Err(format!("FMP historical: HTTP {}", resp.status()));
    }
    let v: serde_json::Value = resp
        .json()
        .await
        .map_err(|e| format!("FMP historical parse: {e}"))?;
    let mut rows = Vec::new();
    if let Some(arr) = v["historical"].as_array() {
        for e in arr.iter().take(limit.max(1)) {
            rows.push(HistoricalPriceRow {
                date: e["date"].as_str().unwrap_or("").to_string(),
                open: e["open"].as_f64().unwrap_or(0.0),
                high: e["high"].as_f64().unwrap_or(0.0),
                low: e["low"].as_f64().unwrap_or(0.0),
                close: e["close"].as_f64().unwrap_or(0.0),
                adj_close: e["adjClose"].as_f64().unwrap_or(0.0),
                volume: e["volume"].as_f64().unwrap_or(0.0),
                change: e["change"].as_f64().unwrap_or(0.0),
                change_pct: e["changePercent"].as_f64().unwrap_or(0.0),
            });
        }
    }
    Ok(rows)
}

/// FMP /v3/earning_surprise/{symbol} — quarterly actual-vs-estimate EPS history.
pub async fn fetch_fmp_earnings_surprises(
    client: &reqwest::Client,
    symbol: &str,
    fmp_key: &str,
) -> Result<Vec<EarningsSurprise>, String> {
    if fmp_key.is_empty() {
        return Err("FMP API key required".into());
    }
    let url = format!(
        "https://financialmodelingprep.com/api/v3/earning_surprise/{}?apikey={}",
        symbol, fmp_key
    );
    let resp = client
        .get(&url)
        .send()
        .await
        .map_err(|e| format!("FMP surprise failed: {e}"))?;
    if !resp.status().is_success() {
        return Err(format!("FMP surprise: HTTP {}", resp.status()));
    }
    let arr: Vec<serde_json::Value> = resp
        .json()
        .await
        .map_err(|e| format!("FMP surprise parse: {e}"))?;
    let rows = arr
        .into_iter()
        .map(|e| {
            let actual = e["actualEarningResult"].as_f64().unwrap_or(0.0);
            let est = e["estimatedEarning"].as_f64().unwrap_or(0.0);
            let surprise = actual - est;
            let surprise_pct = if est.abs() > 1e-9 {
                (surprise / est.abs()) * 100.0
            } else {
                0.0
            };
            EarningsSurprise {
                date: e["date"].as_str().unwrap_or("").to_string(),
                symbol: e["symbol"].as_str().unwrap_or(symbol).to_uppercase(),
                eps_actual: actual,
                eps_estimate: est,
                surprise,
                surprise_pct,
            }
        })
        .collect();
    Ok(rows)
}

// ── fetchers ──

/// Yahoo batch-quote the WORLD_INDICES_UNIVERSE tickers for the WEI dashboard.
/// Returns rows in the universe's declared order so the UI grouping stays stable.
pub async fn fetch_world_indices(client: &reqwest::Client) -> Result<Vec<WorldIndex>, String> {
    let tickers: Vec<&str> = WORLD_INDICES_UNIVERSE.iter().map(|(t, _, _)| *t).collect();
    let quotes = fetch_yahoo_quotes(client, &tickers).await?;
    let mut by_sym: std::collections::HashMap<String, (f64, f64, f64)> =
        std::collections::HashMap::new();
    for (sym, price, change, pct) in quotes {
        by_sym.insert(sym, (price, change, pct));
    }
    let rows: Vec<WorldIndex> = WORLD_INDICES_UNIVERSE
        .iter()
        .map(|(t, d, r)| {
            let (price, change, pct) = by_sym.get(*t).cloned().unwrap_or((0.0, 0.0, 0.0));
            WorldIndex {
                ticker: (*t).to_string(),
                display: (*d).to_string(),
                region: (*r).to_string(),
                price,
                change,
                change_pct: pct,
            }
        })
        .collect();
    Ok(rows)
}

/// Helper — parse a single FMP mover row into MarketMover.
pub(super) fn parse_fmp_mover(e: &serde_json::Value) -> MarketMover {
    let price = e["price"].as_f64().unwrap_or(0.0);
    let change = e["change"]
        .as_f64()
        .or_else(|| e["changes"].as_f64())
        .unwrap_or(0.0);
    // FMP often returns "changesPercentage" as a string like "-5.60%"
    let change_pct = e["changesPercentage"]
        .as_f64()
        .or_else(|| {
            e["changesPercentage"].as_str().map(|s| {
                s.trim_matches(|c: char| c == '%' || c.is_whitespace())
                    .parse::<f64>()
                    .unwrap_or(0.0)
            })
        })
        .unwrap_or(0.0);
    MarketMover {
        symbol: e["symbol"].as_str().unwrap_or("").to_string(),
        name: e["name"].as_str().unwrap_or("").to_string(),
        price,
        change,
        change_pct,
        volume: e["volume"].as_f64().unwrap_or(0.0),
    }
}

/// FMP /v3/stock_market/{gainers|losers|actives} — bundled into one MarketMovers.
pub async fn fetch_fmp_market_movers(
    client: &reqwest::Client,
    fmp_key: &str,
) -> Result<MarketMovers, String> {
    if fmp_key.is_empty() {
        return Err("FMP API key required".into());
    }
    let mut out = MarketMovers::default();
    for (bucket, field) in [("gainers", 0), ("losers", 1), ("actives", 2)] {
        let url = format!(
            "https://financialmodelingprep.com/api/v3/stock_market/{}?apikey={}",
            bucket, fmp_key
        );
        let resp = client
            .get(&url)
            .send()
            .await
            .map_err(|e| format!("FMP {} failed: {}", bucket, e))?;
        if !resp.status().is_success() {
            return Err(format!("FMP {}: HTTP {}", bucket, resp.status()));
        }
        let arr: Vec<serde_json::Value> = resp
            .json()
            .await
            .map_err(|e| format!("FMP {} parse: {}", bucket, e))?;
        let rows: Vec<MarketMover> = arr.iter().map(parse_fmp_mover).collect();
        match field {
            0 => out.gainers = rows,
            1 => out.losers = rows,
            _ => out.actives = rows,
        }
    }
    Ok(out)
}

/// FMP /v3/sector-performance — intraday performance for all GICS sectors.
pub async fn fetch_fmp_sector_performance(
    client: &reqwest::Client,
    fmp_key: &str,
) -> Result<Vec<SectorPerformance>, String> {
    if fmp_key.is_empty() {
        return Err("FMP API key required".into());
    }
    let url = format!(
        "https://financialmodelingprep.com/api/v3/sector-performance?apikey={}",
        fmp_key
    );
    let resp = client
        .get(&url)
        .send()
        .await
        .map_err(|e| format!("FMP sector-performance failed: {e}"))?;
    if !resp.status().is_success() {
        return Err(format!("FMP sector-performance: HTTP {}", resp.status()));
    }
    let arr: Vec<serde_json::Value> = resp
        .json()
        .await
        .map_err(|e| format!("FMP sector-performance parse: {e}"))?;
    let rows: Vec<SectorPerformance> = arr
        .into_iter()
        .map(|e| {
            let sector = e["sector"].as_str().unwrap_or("").to_string();
            // FMP returns "changesPercentage" as a "1.23%" string.
            let pct_raw = e["changesPercentage"].as_str().unwrap_or("0");
            let change_pct = pct_raw
                .trim_matches(|c: char| c == '%' || c.is_whitespace())
                .parse::<f64>()
                .unwrap_or(0.0);
            SectorPerformance { sector, change_pct }
        })
        .collect();
    Ok(rows)
}

async fn fetch_yahoo_options_payload(
    client: &reqwest::Client,
    symbol: &str,
    expiration_ts: Option<i64>,
) -> Result<serde_json::Value, String> {
    let mut url = format!(
        "https://query2.finance.yahoo.com/v7/finance/options/{}",
        symbol.to_uppercase()
    );
    if let Some(ts) = expiration_ts {
        url.push_str("?date=");
        url.push_str(&ts.to_string());
    }
    let resp = client
        .get(&url)
        .header(
            "User-Agent",
            "Mozilla/5.0 (X11; Linux x86_64) TyphooN-Terminal/0.1",
        )
        .send()
        .await
        .map_err(|e| format!("Yahoo options request: {e}"))?;
    if !resp.status().is_success() {
        return Err(format!("Yahoo options: HTTP {}", resp.status()));
    }
    resp.json()
        .await
        .map_err(|e| format!("Yahoo options parse: {e}"))
}

fn yahoo_options_result(v: &serde_json::Value) -> Result<&serde_json::Value, String> {
    v.pointer("/optionChain/result/0")
        .ok_or_else(|| "Yahoo options: empty result".to_string())
}

/// Fetch a Yahoo options chain for a symbol. The first request discovers the
/// available expiration timestamps; follow-up `date=` requests then hydrate
/// every expiration Yahoo exposes, bounded to avoid pathological provider
/// responses hanging the OMON/EXPCAL refresh path.
pub async fn fetch_yahoo_options_chain(
    client: &reqwest::Client,
    symbol: &str,
) -> Result<OptionsChainSnapshot, String> {
    const MAX_YAHOO_EXPIRATIONS: usize = 64;

    let first_payload = fetch_yahoo_options_payload(client, symbol, None).await?;
    let first_result = yahoo_options_result(&first_payload)?;
    let today = chrono::Utc::now().format("%Y-%m-%d").to_string();
    let underlying_price = first_result
        .pointer("/quote/regularMarketPrice")
        .and_then(|x| x.as_f64())
        .unwrap_or(0.0);

    let mut expiration_dates: Vec<i64> = first_result
        .get("expirationDates")
        .and_then(|x| x.as_array())
        .map(|arr| arr.iter().filter_map(|v| v.as_i64()).collect())
        .unwrap_or_default();
    expiration_dates.sort_unstable();
    expiration_dates.dedup();

    let first_options = first_result
        .get("options")
        .and_then(|x| x.as_array())
        .and_then(|arr| arr.first())
        .ok_or_else(|| "Yahoo options: options[0] missing".to_string())?;
    let first_expiry = parse_yahoo_option_expiry(first_options, underlying_price);
    let first_exp_ts = first_options
        .get("expirationDate")
        .and_then(|x| x.as_i64())
        .unwrap_or(0);

    if expiration_dates.is_empty() && first_exp_ts > 0 {
        expiration_dates.push(first_exp_ts);
    }

    let mut expirations = Vec::with_capacity(expiration_dates.len().max(1));
    expirations.push(first_expiry);
    let mut failures = Vec::new();

    for exp_ts in expiration_dates
        .iter()
        .copied()
        .filter(|ts| *ts > 0 && *ts != first_exp_ts)
        .take(MAX_YAHOO_EXPIRATIONS.saturating_sub(1))
    {
        match fetch_yahoo_options_payload(client, symbol, Some(exp_ts)).await {
            Ok(payload) => match yahoo_options_result(&payload).and_then(|result| {
                result
                    .get("options")
                    .and_then(|x| x.as_array())
                    .and_then(|arr| arr.first())
                    .ok_or_else(|| "Yahoo options: options[0] missing".to_string())
            }) {
                Ok(options) => {
                    expirations.push(parse_yahoo_option_expiry(options, underlying_price))
                }
                Err(e) => failures.push(format!("{exp_ts}: {e}")),
            },
            Err(e) => failures.push(format!("{exp_ts}: {e}")),
        }
    }

    expirations.sort_by_key(|exp| exp.expiration.clone());
    expirations.dedup_by(|a, b| a.expiration == b.expiration);

    let mut notes = Vec::new();
    if expiration_dates.len() > MAX_YAHOO_EXPIRATIONS {
        notes.push(format!(
            "Yahoo advertised {} expirations; hydrated first {}",
            expiration_dates.len(),
            MAX_YAHOO_EXPIRATIONS
        ));
    } else {
        notes.push(format!("hydrated {} Yahoo expirations", expirations.len()));
    }
    if !failures.is_empty() {
        notes.push(format!(
            "{} expiration fetches failed: {}",
            failures.len(),
            failures.join("; ")
        ));
    }

    Ok(OptionsChainSnapshot {
        symbol: symbol.to_uppercase(),
        as_of: today,
        underlying_price,
        expirations,
        note: notes.join("; "),
    })
}

#[cfg(test)]
mod stock_split_fetch_tests {
    use super::*;

    fn unix_date(date: &str) -> i64 {
        chrono::NaiveDate::parse_from_str(date, "%Y-%m-%d")
            .unwrap()
            .and_hms_opt(0, 0, 0)
            .unwrap()
            .and_utc()
            .timestamp()
    }

    #[test]
    fn yahoo_split_parser_reads_reverse_split_events() {
        let payload = serde_json::json!({
            "chart": {
                "result": [{
                    "events": {
                        "splits": {
                            "wok_20260618": {
                                "date": unix_date("2026-06-18"),
                                "numerator": 1.0,
                                "denominator": 100.0,
                                "splitRatio": "1:100"
                            }
                        }
                    }
                }]
            }
        });

        let rows = parse_yahoo_stock_splits_value(&payload).unwrap();
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].date, "2026-06-18");
        assert_eq!(rows[0].numerator, 1.0);
        assert_eq!(rows[0].denominator, 100.0);
    }

    #[test]
    fn yahoo_split_parser_falls_back_to_split_ratio_text() {
        let payload = serde_json::json!({
            "chart": {
                "result": [{
                    "events": {
                        "splits": {
                            "ratio_only": {
                                "date": unix_date("2026-06-18"),
                                "splitRatio": "1:100"
                            }
                        }
                    }
                }]
            }
        });

        let rows = parse_yahoo_stock_splits_value(&payload).unwrap();
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].date, "2026-06-18");
        assert_eq!(rows[0].numerator, 1.0);
        assert_eq!(rows[0].denominator, 100.0);
    }
}
