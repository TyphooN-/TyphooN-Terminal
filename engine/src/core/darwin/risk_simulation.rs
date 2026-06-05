use rusqlite::Connection;
use serde::{Deserialize, Serialize};

use super::*;

// ── Xorshift64 RNG ──────────────────────────────────────────────────

pub(super) struct Xorshift64 {
    state: u64,
}

impl Xorshift64 {
    pub(super) fn new(seed: u64) -> Self {
        Self {
            state: if seed == 0 {
                0xDEAD_BEEF_CAFE_1337
            } else {
                seed
            },
        }
    }

    pub(super) fn next_u64(&mut self) -> u64 {
        let mut x = self.state;
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        self.state = x;
        x
    }

    /// Returns a usize in [0, n)
    pub(super) fn next_usize(&mut self, n: usize) -> usize {
        (self.next_u64() % n as u64) as usize
    }
}

// ── Monte Carlo VaR Simulation ─────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MonteCarloResult {
    pub simulations: i64,
    pub days_forward: i64,
    pub var_95: f64,
    pub var_99: f64,
    pub median_outcome: f64,
    pub worst_case: f64,
    pub best_case: f64,
    pub probability_of_loss: f64,
    pub percentiles: Vec<(i32, f64)>,
}

/// Run Monte Carlo simulation using daily return distribution.
/// Randomly samples (with replacement) `days_forward` daily returns per path,
/// cumulates them, and computes percentiles across all simulated outcomes.
pub fn monte_carlo_var(
    daily_returns: &[DailyReturn],
    days_forward: i64,
    simulations: i64,
) -> MonteCarloResult {
    let empty = MonteCarloResult {
        simulations,
        days_forward,
        var_95: 0.0,
        var_99: 0.0,
        median_outcome: 0.0,
        worst_case: 0.0,
        best_case: 0.0,
        probability_of_loss: 0.0,
        percentiles: vec![],
    };

    if daily_returns.len() < 2 || simulations <= 0 || days_forward <= 0 {
        return empty;
    }

    let n = daily_returns.len();
    let mut rng = Xorshift64::new(42);
    let mut outcomes: Vec<f64> = Vec::with_capacity(simulations as usize);

    for _ in 0..simulations {
        let mut cumulative = 0.0;
        for _ in 0..days_forward {
            let idx = rng.next_usize(n);
            cumulative += daily_returns[idx].return_pct;
        }
        outcomes.push(cumulative);
    }

    outcomes.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));

    let total = outcomes.len();
    let loss_count = outcomes.iter().filter(|&&x| x < 0.0).count();

    let percentile = |p: f64| -> f64 {
        let idx = ((p / 100.0) * (total as f64 - 1.0)).round() as usize;
        outcomes[idx.min(total - 1)]
    };

    let percentiles_list: Vec<(i32, f64)> = vec![
        (1, percentile(1.0)),
        (5, percentile(5.0)),
        (10, percentile(10.0)),
        (25, percentile(25.0)),
        (50, percentile(50.0)),
        (75, percentile(75.0)),
        (90, percentile(90.0)),
        (95, percentile(95.0)),
        (99, percentile(99.0)),
    ];

    MonteCarloResult {
        simulations,
        days_forward,
        var_95: -percentile(5.0),
        var_99: -percentile(1.0),
        median_outcome: percentile(50.0),
        worst_case: outcomes[0],
        best_case: outcomes[total - 1],
        probability_of_loss: loss_count as f64 / total as f64 * 100.0,
        percentiles: percentiles_list,
    }
}

// ── Stress Test ─────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StressTestResult {
    pub scenario: String,
    pub description: String,
    pub market_drop_pct: f64,
    pub estimated_portfolio_impact: f64,
    pub estimated_portfolio_impact_pct: f64,
}

/// Run stress tests against historical crash scenarios.
/// Estimates portfolio impact based on portfolio beta (correlation with market)
/// scaled by annualized volatility.
pub fn run_stress_tests(conn: &Connection) -> Result<Vec<StressTestResult>, String> {
    let daily_returns = get_portfolio_daily_returns(conn)?;
    if daily_returns.len() < 10 {
        return Err("Insufficient daily returns for stress testing (need >= 10 days)".into());
    }

    let var_stats = compute_var_full(&daily_returns);
    let ann_vol = var_stats.annualized_vol;

    // Estimate portfolio beta: use vol ratio as proxy (portfolio vol / typical market vol ~16%)
    let market_vol = 16.0;
    let beta = if market_vol > 0.0 {
        ann_vol / market_vol
    } else {
        1.0
    };

    // Current portfolio balance (last known)
    let current_balance = daily_returns.last().map(|d| d.balance).unwrap_or(0.0);

    let scenarios = vec![
        (
            "2020 COVID Crash",
            "March 2020: 34% equity drawdown in 23 trading days",
            -34.0,
        ),
        (
            "2022 Rate Hikes",
            "2022 bear market: 25% drawdown over several months",
            -25.0,
        ),
        (
            "2008 GFC",
            "Global Financial Crisis: 57% peak-to-trough equity drawdown",
            -57.0,
        ),
        (
            "Flash Crash",
            "Sudden intraday 10% market drop with rapid partial recovery",
            -10.0,
        ),
        (
            "Tech Wreck 2000",
            "Dot-com bust: 78% drawdown concentrated in growth/tech",
            -78.0,
        ),
        (
            "Crypto Winter",
            "80% drawdown in crypto assets (2018/2022-style bear)",
            -80.0,
        ),
    ];

    let results = scenarios
        .into_iter()
        .map(|(name, desc, drop_pct)| {
            let impact_pct = drop_pct * beta;
            let impact_abs = current_balance * impact_pct / 100.0;
            StressTestResult {
                scenario: name.to_string(),
                description: desc.to_string(),
                market_drop_pct: drop_pct,
                estimated_portfolio_impact: impact_abs,
                estimated_portfolio_impact_pct: impact_pct,
            }
        })
        .collect();

    Ok(results)
}
