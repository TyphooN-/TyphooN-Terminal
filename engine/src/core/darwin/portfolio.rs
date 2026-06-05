use rusqlite::{Connection, params};

use super::*;

// ── Open Position Reconstruction ────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DarwinOpenPosition {
    pub symbol: String,
    pub side: String, // "buy" or "sell"
    pub total_volume: f64,
    pub avg_price: f64,
    pub position_count: i64, // number of individual tickets
    pub notional: f64,       // volume * avg_price
    pub earliest_open: String,
}

/// Reconstruct currently open positions from deals.
/// Uses net volume balance per symbol+side: sum(in volumes) - sum(out volumes).
/// If net > 0, that volume is still open. VWAP computed from "in" deals.
pub fn get_darwin_open_positions(
    conn: &Connection,
    darwin_ticker: &str,
) -> Result<Vec<DarwinOpenPosition>, String> {
    // Aggregate in/out volumes and notional per symbol+side directly in SQL
    // "in" deals add volume, "out" deals subtract volume
    // deal_type for "in" = the side (buy/sell), for "out" = the opposite side
    // So we group by symbol + deal_type on "in" deals only for side detection.
    // prepare_cached: called once per DARWIN account by the BG thread, so the
    // parsed statement is reused across all N accounts per cycle.
    let mut stmt = conn.prepare_cached(
        "SELECT symbol, deal_type, direction, volume, price, time FROM darwin_deals WHERE account = ?1 AND direction IN ('in', 'out') AND symbol != '' ORDER BY time, id"
    ).map_err(|e| format!("Prepare failed: {e}"))?;

    // Track per (symbol, side): net volume, weighted notional for VWAP, deal count, earliest
    struct Agg {
        vol_in: f64,
        vol_out: f64,
        notional_in: f64, // sum of (volume * price) for "in" deals
        count_in: i64,
        earliest: String,
    }

    let mut agg: std::collections::HashMap<(String, String), Agg> =
        std::collections::HashMap::new();

    let rows = stmt
        .query_map(params![darwin_ticker], |row| {
            Ok((
                row.get::<_, String>(0)?, // symbol
                row.get::<_, String>(1)?, // deal_type (buy/sell)
                row.get::<_, String>(2)?, // direction (in/out)
                row.get::<_, f64>(3)?,    // volume
                row.get::<_, f64>(4)?,    // price
                row.get::<_, String>(5)?, // time
            ))
        })
        .map_err(|e| format!("Query failed: {e}"))?;

    for row in rows {
        if let Ok((symbol, deal_type, direction, volume, price, time)) = row {
            if direction == "in" {
                // "in" deal: deal_type is the position side
                let key = (symbol, deal_type);
                let entry = agg.entry(key).or_insert(Agg {
                    vol_in: 0.0,
                    vol_out: 0.0,
                    notional_in: 0.0,
                    count_in: 0,
                    earliest: time.clone(),
                });
                entry.vol_in += volume;
                entry.notional_in += volume * price;
                entry.count_in += 1;
                if time < entry.earliest {
                    entry.earliest = time.clone();
                }
            } else if direction == "out" {
                // "out" deal: deal_type is OPPOSITE of position side
                // buy out = closing a sell position, sell out = closing a buy position
                let side = if deal_type == "buy" { "sell" } else { "buy" };
                let key = (symbol, side.to_string());
                if let Some(entry) = agg.get_mut(&key) {
                    entry.vol_out += volume;
                }
            }
        }
    }

    let mut result: Vec<DarwinOpenPosition> = Vec::new();
    for ((symbol, side), a) in agg {
        let net_vol = a.vol_in - a.vol_out;
        if net_vol <= 0.0 {
            continue;
        } // fully closed
        let avg_price = if a.vol_in > 0.0 {
            a.notional_in / a.vol_in
        } else {
            0.0
        };
        let notional = net_vol * avg_price;
        result.push(DarwinOpenPosition {
            symbol,
            side,
            total_volume: net_vol,
            avg_price,
            position_count: a.count_in, // total "in" deal count (some may be closed)
            notional,
            earliest_open: a.earliest,
        });
    }

    // Sort by notional descending
    result.sort_by(|a, b| {
        b.notional
            .partial_cmp(&a.notional)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    Ok(result)
}

// ── Combined Portfolio Analytics ────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PortfolioSummary {
    pub accounts: Vec<DarwinAccountSummary>,
    pub total_initial_balance: f64,
    pub total_final_balance: f64,
    pub total_net_pnl: f64,
    pub total_commission: f64,
    pub total_deals: i64,
    pub total_positions: i64,
    pub combined_max_drawdown_pct: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PortfolioOpenPosition {
    pub symbol: String,
    pub side: String,
    pub total_volume: f64,
    pub avg_price: f64,
    pub notional: f64,
    pub darwin_breakdown: Vec<(String, f64, f64)>, // (ticker, volume, avg_price)
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PortfolioSymbolExposure {
    pub symbol: String,
    pub long_notional: f64,
    pub short_notional: f64,
    pub net_notional: f64,
    pub darwin_count: i64,    // how many DARWINs hold this symbol
    pub darwins: Vec<String>, // which DARWINs
}

/// Get combined portfolio summary across all DARWINs.
pub fn get_portfolio_summary(conn: &Connection) -> Result<PortfolioSummary, String> {
    let accounts = list_darwin_accounts(conn)?;
    let mut summaries = Vec::new();
    let mut total_initial = 0.0f64;
    let mut total_final = 0.0f64;
    let mut total_pnl = 0.0f64;
    let mut total_comm = 0.0f64;
    let mut total_deals = 0i64;
    let mut total_positions = 0i64;

    for account in &accounts {
        match get_darwin_summary(conn, &account.darwin_ticker) {
            Ok(s) => {
                total_initial += s.account.initial_balance;
                total_final += s.final_balance;
                total_pnl += s.total_profit + s.total_commission + s.total_swap;
                total_comm += s.total_commission;
                total_deals += s.account.deal_count;
                total_positions += s.account.position_count;
                summaries.push(s);
            }
            Err(_) => continue,
        }
    }

    // Combined max drawdown from aggregate equity curve
    let combined_dd = get_portfolio_max_drawdown(conn).unwrap_or(0.0);

    Ok(PortfolioSummary {
        accounts: summaries,
        total_initial_balance: total_initial,
        total_final_balance: total_final,
        total_net_pnl: total_pnl,
        total_commission: total_comm,
        total_deals,
        total_positions,
        combined_max_drawdown_pct: combined_dd,
    })
}

/// Get combined open positions across all DARWINs, aggregated by symbol.
pub fn get_portfolio_open_positions(
    conn: &Connection,
) -> Result<Vec<PortfolioOpenPosition>, String> {
    let accounts = list_darwin_accounts(conn)?;

    // (symbol, side) -> vec of (ticker, volume, avg_price)
    let mut combined: std::collections::HashMap<(String, String), Vec<(String, f64, f64)>> =
        std::collections::HashMap::new();

    for account in &accounts {
        if let Ok(positions) = get_darwin_open_positions(conn, &account.darwin_ticker) {
            for p in positions {
                let key = (p.symbol.clone(), p.side.clone());
                combined.entry(key).or_default().push((
                    account.darwin_ticker.clone(),
                    p.total_volume,
                    p.avg_price,
                ));
            }
        }
    }

    let mut result: Vec<PortfolioOpenPosition> = combined
        .into_iter()
        .map(|((symbol, side), entries)| {
            let total_vol: f64 = entries.iter().map(|(_, v, _)| v).sum();
            let total_notional: f64 = entries.iter().map(|(_, v, p)| v * p).sum();
            let avg_price = if total_vol > 0.0 {
                total_notional / total_vol
            } else {
                0.0
            };
            PortfolioOpenPosition {
                symbol,
                side,
                total_volume: total_vol,
                avg_price,
                notional: total_notional,
                darwin_breakdown: entries,
            }
        })
        .collect();

    result.sort_by(|a, b| {
        b.notional
            .partial_cmp(&a.notional)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    Ok(result)
}

/// Get symbol exposure across all DARWINs (long + short per symbol).
pub fn get_portfolio_exposure(conn: &Connection) -> Result<Vec<PortfolioSymbolExposure>, String> {
    let accounts = list_darwin_accounts(conn)?;

    // symbol -> (long_notional, short_notional, darwins_set)
    let mut exposure: std::collections::HashMap<
        String,
        (f64, f64, std::collections::HashSet<String>),
    > = std::collections::HashMap::new();

    for account in &accounts {
        if let Ok(positions) = get_darwin_open_positions(conn, &account.darwin_ticker) {
            for p in positions {
                let entry = exposure.entry(p.symbol.clone()).or_insert((
                    0.0,
                    0.0,
                    std::collections::HashSet::new(),
                ));
                if p.side == "buy" {
                    entry.0 += p.notional;
                } else {
                    entry.1 += p.notional;
                }
                entry.2.insert(account.darwin_ticker.clone());
            }
        }
    }

    let mut result: Vec<PortfolioSymbolExposure> = exposure
        .into_iter()
        .map(|(symbol, (long, short, darwins))| {
            let mut d: Vec<String> = darwins.into_iter().collect();
            d.sort();
            PortfolioSymbolExposure {
                symbol,
                long_notional: long,
                short_notional: short,
                net_notional: long - short,
                darwin_count: d.len() as i64,
                darwins: d,
            }
        })
        .collect();

    // Sort by absolute net notional descending
    result.sort_by(|a, b| {
        b.net_notional
            .abs()
            .partial_cmp(&a.net_notional.abs())
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    Ok(result)
}

/// Get combined equity curve across all DARWINs (daily aggregate).
/// Single SQL query instead of N per-account queries.
pub fn get_portfolio_equity_curve(conn: &Connection) -> Result<Vec<(String, f64)>, String> {
    // prepare_cached: called every BG cycle (portfolio-level refresh).
    let mut stmt = conn
        .prepare_cached(
            "SELECT SUBSTR(d.time, 1, 10) as date, SUM(d.balance) as total_balance
         FROM darwin_deals d
         INNER JOIN (
           SELECT account, SUBSTR(time, 1, 10) as day, MAX(id) as max_id
           FROM darwin_deals
           WHERE balance > 0
           GROUP BY account, SUBSTR(time, 1, 10)
         ) g ON d.id = g.max_id
         GROUP BY date
         ORDER BY date",
        )
        .map_err(|e| format!("Prepare failed: {e}"))?;

    let rows = stmt
        .query_map([], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, f64>(1)?))
        })
        .map_err(|e| format!("Query failed: {e}"))?;

    let mut result = Vec::new();
    for row in rows {
        if let Ok(point) = row {
            result.push(point);
        }
    }
    Ok(result)
}

fn get_portfolio_max_drawdown(conn: &Connection) -> Result<f64, String> {
    let curve = get_portfolio_equity_curve(conn)?;
    let mut peak = 0.0f64;
    let mut max_dd = 0.0f64;
    for (_, balance) in &curve {
        if *balance > peak {
            peak = *balance;
        }
        if peak > 0.0 {
            let dd = (peak - balance) / peak * 100.0;
            if dd > max_dd {
                max_dd = dd;
            }
        }
    }
    Ok(max_dd)
}
