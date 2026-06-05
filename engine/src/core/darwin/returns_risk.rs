use rusqlite::{Connection, params};

use super::*;

// ── Advanced Analytics ──────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DailyReturn {
    pub date: String,
    pub pnl: f64,
    pub balance: f64,
    pub return_pct: f64,
    pub drawdown_pct: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VaRResult {
    pub var_95: f64,
    pub var_99: f64,
    pub cvar_95: f64, // conditional VaR (expected shortfall)
    pub cvar_99: f64,
    pub daily_vol: f64, // daily volatility
    pub annualized_vol: f64,
    pub sharpe: f64,
    pub sortino: f64,
    pub calmar: f64,
    pub max_drawdown_pct: f64,
    pub avg_daily_pnl: f64,
    pub worst_day: f64,
    pub best_day: f64,
    pub trading_days: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MonthlyReturn {
    pub year: i32,
    pub month: i32,
    pub pnl: f64,
    pub return_pct: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RollingVaR {
    pub date: String,
    pub var_95: f64,
    pub var_99: f64,
    pub rolling_sharpe: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CorrelationEntry {
    pub darwin_a: String,
    pub darwin_b: String,
    pub correlation: f64,
}

/// Get daily returns from deal balance changes for a DARWIN.
/// Uses SQL GROUP BY to deduplicate at the DB level instead of loading all 62K deals.
pub fn get_daily_returns(
    conn: &Connection,
    darwin_ticker: &str,
) -> Result<Vec<DailyReturn>, String> {
    // SQL-level dedup: get last balance per day using MAX(id) subquery.
    // prepare_cached: this function is called N times per BG cycle (once per
    // account inside get_darwin_correlations / get_portfolio_daily_returns).
    // Caching the prepared statement reuses the parse across all N calls.
    let mut stmt = conn
        .prepare_cached(
            "SELECT SUBSTR(d.time, 1, 10) as date, d.balance
         FROM darwin_deals d
         INNER JOIN (
           SELECT SUBSTR(time, 1, 10) as day, MAX(id) as max_id
           FROM darwin_deals
           WHERE account = ?1 AND balance > 0
           GROUP BY SUBSTR(time, 1, 10)
         ) g ON d.id = g.max_id
         ORDER BY date",
        )
        .map_err(|e| format!("Prepare failed: {e}"))?;

    let daily_balances: Vec<(String, f64)> = stmt
        .query_map(params![darwin_ticker], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, f64>(1)?))
        })
        .map_err(|e| format!("Query failed: {e}"))?
        .filter_map(|r| r.ok())
        .collect();

    // Compute returns and drawdown
    let mut result = Vec::new();
    let mut peak = 0.0f64;
    for i in 0..daily_balances.len() {
        let (ref date, balance) = daily_balances[i];
        let prev_balance = if i > 0 {
            daily_balances[i - 1].1
        } else {
            balance
        };
        let pnl = balance - prev_balance;
        let return_pct = if prev_balance > 0.0 {
            pnl / prev_balance * 100.0
        } else {
            0.0
        };
        if balance > peak {
            peak = balance;
        }
        let drawdown_pct = if peak > 0.0 {
            (peak - balance) / peak * 100.0
        } else {
            0.0
        };

        result.push(DailyReturn {
            date: date.clone(),
            pnl,
            balance,
            return_pct,
            drawdown_pct,
        });
    }
    Ok(result)
}

/// Compute VaR and risk metrics for a DARWIN or portfolio.
/// Compute VaR using a minimum 20-day (1 month) window.
/// For Darwinex-specific VaR, use compute_var_multipliers() which uses 45d+6m blend.
/// This function uses the last 20+ days if available, full history if less than 20 days.
pub fn compute_var(daily_returns: &[DailyReturn]) -> VaRResult {
    // Use at least 20 trading days (1 month) for meaningful VaR
    let windowed = if daily_returns.len() > 20 {
        &daily_returns[daily_returns.len() - 20..]
    } else {
        daily_returns
    };
    compute_var_full(windowed)
}

/// Compute VaR on the full provided dataset (no windowing).
/// Used internally and by functions that pre-window their data.
pub fn compute_var_full(daily_returns: &[DailyReturn]) -> VaRResult {
    if daily_returns.len() < 2 {
        return VaRResult {
            var_95: 0.0,
            var_99: 0.0,
            cvar_95: 0.0,
            cvar_99: 0.0,
            daily_vol: 0.0,
            annualized_vol: 0.0,
            sharpe: 0.0,
            sortino: 0.0,
            calmar: 0.0,
            max_drawdown_pct: 0.0,
            avg_daily_pnl: 0.0,
            worst_day: 0.0,
            best_day: 0.0,
            trading_days: 0,
        };
    }

    // Single fused pass: accumulate pnl sum, return sum, return sum_sq,
    // downside sum_sq + count, and max drawdown while collecting pnls for sort.
    // Previously built TWO Vec<f64>, then took 4 additional passes for sums/vars.
    let n_u = daily_returns.len();
    let n = n_u as f64;
    let mut pnls: Vec<f64> = Vec::with_capacity(n_u);
    let mut sum_pnl = 0.0f64;
    let mut sum_ret = 0.0f64;
    let mut sum_ret_sq = 0.0f64;
    let mut downside_sq = 0.0f64;
    let mut downside_count = 0usize;
    let mut max_dd = 0.0f64;
    for r in daily_returns {
        pnls.push(r.pnl);
        sum_pnl += r.pnl;
        sum_ret += r.return_pct;
        sum_ret_sq += r.return_pct * r.return_pct;
        if r.return_pct < 0.0 {
            downside_sq += r.return_pct * r.return_pct;
            downside_count += 1;
        }
        if r.drawdown_pct > max_dd {
            max_dd = r.drawdown_pct;
        }
    }
    let avg_pnl = sum_pnl / n;
    let avg_ret = sum_ret / n;

    // Sample variance via the raw-moments formula: Σx² − n·mean² / (n−1)
    let variance = ((sum_ret_sq - n * avg_ret * avg_ret) / (n - 1.0)).max(0.0);
    let daily_vol = variance.sqrt();
    let annualized_vol = daily_vol * (252.0f64).sqrt();

    // Downside deviation (for Sortino)
    let downside_dev = {
        let denom = downside_count.max(1) as f64;
        (downside_sq / denom).sqrt()
    };

    // Sort pnls in place for percentile VaR / CVaR / worst / best.
    let mut sorted_pnls = pnls;
    sorted_pnls.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));

    let idx_95 = ((n) * 0.05).floor() as usize;
    let idx_99 = ((n) * 0.01).floor() as usize;

    let var_95 = sorted_pnls.get(idx_95).copied().unwrap_or(0.0).abs();
    let var_99 = sorted_pnls.get(idx_99).copied().unwrap_or(0.0).abs();

    // CVaR (expected shortfall) = average of losses beyond VaR
    let cvar_95 = if idx_95 > 0 {
        sorted_pnls[..idx_95].iter().sum::<f64>() / idx_95 as f64
    } else {
        sorted_pnls[0]
    }
    .abs();
    let cvar_99 = if idx_99 > 0 {
        sorted_pnls[..idx_99].iter().sum::<f64>() / idx_99 as f64
    } else {
        sorted_pnls[0]
    }
    .abs();

    let worst = sorted_pnls.first().copied().unwrap_or(0.0);
    let best = sorted_pnls.last().copied().unwrap_or(0.0);

    // Sharpe (annualized, risk-free = 0)
    let sharpe = if daily_vol > 0.0 {
        avg_ret / daily_vol * (252.0f64).sqrt()
    } else {
        0.0
    };
    let sortino = if downside_dev > 0.0 {
        avg_ret / downside_dev * (252.0f64).sqrt()
    } else if avg_ret > 0.0 {
        99.0 // Perfect record: positive returns with no downside volatility
    } else {
        0.0
    };

    // Calmar (annualized return / max drawdown)
    let annualized_return = avg_ret * 252.0;
    let calmar = if max_dd > 0.0 {
        annualized_return / max_dd
    } else if annualized_return > 0.0 {
        99.0 // Perfect record: positive return with no drawdown
    } else {
        0.0
    };

    VaRResult {
        var_95,
        var_99,
        cvar_95,
        cvar_99,
        daily_vol,
        annualized_vol,
        sharpe,
        sortino,
        calmar,
        max_drawdown_pct: max_dd,
        avg_daily_pnl: avg_pnl,
        worst_day: worst,
        best_day: best,
        trading_days: n_u as i64,
    }
}

/// Get monthly returns for a DARWIN.
pub fn get_monthly_returns(daily_returns: &[DailyReturn]) -> Vec<MonthlyReturn> {
    let mut monthly: std::collections::BTreeMap<(i32, i32), (f64, f64, f64)> =
        std::collections::BTreeMap::new();
    // (year, month) -> (total_pnl, start_balance, end_balance)

    for r in daily_returns {
        if r.date.len() < 7 {
            continue;
        }
        let year: i32 = r.date[..4].parse().unwrap_or(0);
        let month: i32 = r.date[5..7].parse().unwrap_or(0);
        if year == 0 || month == 0 {
            continue;
        }
        let entry = monthly
            .entry((year, month))
            .or_insert((0.0, r.balance - r.pnl, r.balance));
        entry.0 += r.pnl;
        entry.2 = r.balance; // update end balance
    }

    monthly
        .into_iter()
        .map(|((year, month), (pnl, start, _end))| {
            let return_pct = if start > 0.0 {
                pnl / start * 100.0
            } else {
                0.0
            };
            MonthlyReturn {
                year,
                month,
                pnl,
                return_pct,
            }
        })
        .collect()
}

/// Compute rolling VaR (window_days lookback).
pub fn get_rolling_var(daily_returns: &[DailyReturn], window_days: usize) -> Vec<RollingVaR> {
    if daily_returns.len() < window_days {
        return Vec::new();
    }

    let mut result = Vec::with_capacity(daily_returns.len() - window_days);
    // PERF: reuse the pnls buffer across windows — was allocating a fresh Vec
    // per iteration. For ~500 days × 30-day window that's 500 extra allocations.
    let mut pnls: Vec<f64> = Vec::with_capacity(window_days);
    let sqrt_252 = (252.0_f64).sqrt();
    for i in window_days..daily_returns.len() {
        let window = &daily_returns[i - window_days..i];
        pnls.clear();
        pnls.extend(window.iter().map(|r| r.pnl));
        pnls.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));

        let idx_95 = ((pnls.len() as f64) * 0.05).floor() as usize;
        let idx_99 = ((pnls.len() as f64) * 0.01).floor() as usize;
        let var_95 = pnls.get(idx_95).copied().unwrap_or(0.0).abs();
        let var_99 = pnls.get(idx_99).copied().unwrap_or(0.0).abs();

        // Single-pass mean + variance over return_pct (avoid a second Vec + 2 passes).
        let mut sum = 0.0f64;
        let mut sum_sq = 0.0f64;
        let n = window.len() as f64;
        for r in window {
            sum += r.return_pct;
            sum_sq += r.return_pct * r.return_pct;
        }
        let avg = sum / n;
        let variance = (sum_sq - sum * avg) / (n - 1.0);
        let vol = variance.max(0.0).sqrt();
        let sharpe = if vol > 0.0 { avg / vol * sqrt_252 } else { 0.0 };

        result.push(RollingVaR {
            date: daily_returns[i].date.clone(),
            var_95,
            var_99,
            rolling_sharpe: sharpe,
        });
    }
    result
}

/// Compute cross-DARWIN correlation matrix from daily returns.
/// Uses a 45-day rolling window to match Darwinex's correlation methodology.
/// Darwinex calculates DARWIN correlation based on the last 45 trading days of returns.
pub fn get_darwin_correlations(conn: &Connection) -> Result<Vec<CorrelationEntry>, String> {
    let accounts = list_darwin_accounts(conn)?;
    let mut all_returns: Vec<(String, std::collections::HashMap<String, f64>)> = Vec::new();

    for account in &accounts {
        let returns = get_daily_returns(conn, &account.darwin_ticker)?;
        // Use only last 45 trading days (Darwinex standard correlation window)
        let recent = if returns.len() > 45 {
            &returns[returns.len() - 45..]
        } else {
            &returns
        };
        let map: std::collections::HashMap<String, f64> = recent
            .iter()
            .map(|r| (r.date.clone(), r.return_pct))
            .collect();
        all_returns.push((account.darwin_ticker.clone(), map));
    }

    // Pre-compute per-series statistics in a single pass (Welford-style)
    // For each series: collect returns as Vec for common-date pairing,
    // and pre-compute mean + variance to avoid redundant iterations.
    let stats: Vec<(String, std::collections::HashMap<String, f64>)> = all_returns;

    let mut result = Vec::with_capacity(stats.len() * stats.len());
    for i in 0..stats.len() {
        for j in i..stats.len() {
            let (ref name_a, ref map_a) = stats[i];
            let (ref name_b, ref map_b) = stats[j];

            // Pearson correlation over common dates only.
            // PERF: single-pass running-sums formula — was allocating a
            // `Vec<(f64, f64)>` and then making three passes over it
            // (mean_a, mean_b, then cov/var_a/var_b). Here we accumulate the
            // sums in one pass with zero intermediate allocation, then derive
            // cov and variance from the sums at the end.
            let corr = if i == j {
                1.0
            } else {
                let mut sum_a = 0.0f64;
                let mut sum_b = 0.0f64;
                let mut sum_aa = 0.0f64;
                let mut sum_bb = 0.0f64;
                let mut sum_ab = 0.0f64;
                let mut n = 0i32;
                // Iterate the smaller map for cheaper lookups.
                let (iter_map, other_map) = if map_a.len() <= map_b.len() {
                    (map_a, map_b)
                } else {
                    (map_b, map_a)
                };
                for (date, &va) in iter_map {
                    if let Some(&vb) = other_map.get(date) {
                        sum_a += va;
                        sum_b += vb;
                        sum_aa += va * va;
                        sum_bb += vb * vb;
                        sum_ab += va * vb;
                        n += 1;
                    }
                }
                if n > 2 {
                    let nf = n as f64;
                    let cov = sum_ab - sum_a * sum_b / nf;
                    let var_a = sum_aa - sum_a * sum_a / nf;
                    let var_b = sum_bb - sum_b * sum_b / nf;
                    if var_a > 0.0 && var_b > 0.0 {
                        (cov / (var_a.sqrt() * var_b.sqrt())).clamp(-1.0, 1.0)
                    } else {
                        0.0
                    }
                } else {
                    0.0
                }
            };

            result.push(CorrelationEntry {
                darwin_a: name_a.clone(),
                darwin_b: name_b.clone(),
                correlation: corr,
            });
            if i != j {
                result.push(CorrelationEntry {
                    darwin_a: name_b.clone(),
                    darwin_b: name_a.clone(),
                    correlation: corr,
                });
            }
        }
    }
    Ok(result)
}

/// Get combined daily returns across all DARWINs (portfolio-level).
pub fn get_portfolio_daily_returns(conn: &Connection) -> Result<Vec<DailyReturn>, String> {
    let accounts = list_darwin_accounts(conn)?;
    let mut combined: std::collections::BTreeMap<String, (f64, f64)> =
        std::collections::BTreeMap::new(); // date -> (total_pnl, total_balance)

    for account in &accounts {
        let returns = get_daily_returns(conn, &account.darwin_ticker)?;
        for r in &returns {
            let entry = combined.entry(r.date.clone()).or_insert((0.0, 0.0));
            entry.0 += r.pnl;
            entry.1 += r.balance;
        }
    }

    let mut result = Vec::new();
    let mut peak = 0.0f64;
    for (date, (pnl, balance)) in &combined {
        let prev_balance = balance - pnl;
        let return_pct = if prev_balance > 0.0 {
            pnl / prev_balance * 100.0
        } else {
            0.0
        };
        if *balance > peak {
            peak = *balance;
        }
        let drawdown_pct = if peak > 0.0 {
            (peak - balance) / peak * 100.0
        } else {
            0.0
        };
        result.push(DailyReturn {
            date: date.clone(),
            pnl: *pnl,
            balance: *balance,
            return_pct,
            drawdown_pct,
        });
    }
    Ok(result)
}
