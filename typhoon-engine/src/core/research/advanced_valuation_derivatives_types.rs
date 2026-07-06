use serde::{Deserialize, Serialize};

// HRA, DCF, SVM, options-chain, and implied-volatility research types
// HRA / DCF / SVM / OMON / IVOL surfaces.

/// HRA — one rolling-period return row (e.g. 1M, 3M, 1Y, YTD).
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct HraWindow {
    pub label: String, // "1D" / "5D" / "1M" / "3M" / "6M" / "YTD" / "1Y" / "3Y" / "5Y" / "ITD"
    pub trading_days: usize, // 0 for YTD/ITD which span by date
    pub return_pct: f64, // simple return (pct)
    pub cagr_pct: f64, // annualized when trading_days > 252
    pub n_observations: usize,
}

/// HRA — historical return + risk snapshot for a symbol.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct HraSnapshot {
    pub symbol: String,
    pub as_of: String, // YYYY-MM-DD
    pub last_close: f64,
    pub windows: Vec<HraWindow>,
    pub max_drawdown_pct: f64, // ITD, negative number
    pub drawdown_peak_date: String,
    pub drawdown_trough_date: String,
    pub volatility_annual_pct: f64, // stdev of daily log-returns × sqrt(252) × 100
    pub sharpe_ratio: f64,          // (mean daily return - rf) / stdev, annualized
    pub sortino_ratio: f64,         // same but downside deviation denominator
    pub calmar_ratio: f64,          // CAGR / |max_drawdown|
    pub risk_free_pct: f64,         // used in Sharpe/Sortino
    pub note: String,
}

/// DCF — one projection year in the explicit forecast period.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct DcfYear {
    pub year: i32, // calendar year or offset
    pub revenue: f64,
    pub ebit: f64,
    pub nopat: f64, // NOPAT = EBIT × (1 - t)
    pub fcff: f64,  // free cash flow to firm
    pub discount_factor: f64,
    pub pv_fcff: f64, // fcff × discount_factor
}

/// DCF — Discounted Cash Flow fair value snapshot.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct DcfSnapshot {
    pub symbol: String,
    pub as_of: String,
    pub method: String, // "DCF on FCFF"
    pub base_revenue: f64,
    pub base_fcff: f64,
    pub growth_pct: f64,          // explicit-period revenue growth
    pub terminal_growth_pct: f64, // Gordon growth in perpetuity
    pub wacc_pct: f64,            // discount rate
    pub tax_rate_pct: f64,
    pub fcff_margin_pct: f64, // fcff / revenue applied to projections
    pub projection_years: usize,
    pub years: Vec<DcfYear>,
    pub pv_sum: f64,           // Σ pv of explicit FCFF
    pub terminal_value: f64,   // TV at end of explicit period
    pub pv_terminal: f64,      // TV × final discount factor
    pub enterprise_value: f64, // pv_sum + pv_terminal
    pub total_debt: f64,
    pub cash_and_equivalents: f64,
    pub equity_value: f64, // EV - debt + cash
    pub shares_outstanding: f64,
    pub implied_price: f64, // equity_value / shares
    pub note: String,
}

/// SVM — one row in the multi-model fair-value triangulation.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SvmModelRow {
    pub model: String, // "WACC cost of equity" / "DDM Gordon Growth" / "DCF FCFF" / "RV P/E median" / "RV EV/EBITDA median"
    pub implied_price: f64, // 0.0 if N/A
    pub current_price: f64,
    pub upside_pct: f64,    // (implied / current - 1) × 100
    pub confidence: String, // "high" / "medium" / "low" / "n/a"
    pub source: String,     // short lineage
}

/// SVM — Stock Valuation Model summary for a symbol.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SvmSnapshot {
    pub symbol: String,
    pub as_of: String,
    pub current_price: f64,
    pub rows: Vec<SvmModelRow>,
    pub fair_low: f64,       // min of non-zero implied prices
    pub fair_high: f64,      // max of non-zero implied prices
    pub fair_mid: f64,       // simple mean of non-zero implied prices
    pub upside_mid_pct: f64, // (fair_mid / current - 1) × 100
    pub note: String,
}

/// OMON — one options contract row.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct OptionContract {
    pub contract_symbol: String, // e.g. "AAPL240419C00150000"
    pub option_type: String,     // "CALL" / "PUT"
    pub strike: f64,
    pub last_price: f64,
    pub bid: f64,
    pub ask: f64,
    pub volume: f64,
    pub open_interest: f64,
    pub implied_volatility: f64, // decimal (0.25 = 25%)
    pub in_the_money: bool,
}

/// OMON — one expiration's call+put chain.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct OptionExpiry {
    pub expiration: String, // YYYY-MM-DD
    pub days_to_expiry: i64,
    pub calls: Vec<OptionContract>,
    pub puts: Vec<OptionContract>,
}

/// OMON — complete options-chain snapshot for a symbol.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct OptionsChainSnapshot {
    pub symbol: String,
    pub as_of: String,
    pub underlying_price: f64,
    pub expirations: Vec<OptionExpiry>,
    pub note: String,
}

/// ADR-084 extension: the max-pain strike for one expiration — the strike
/// minimizing the total intrinsic payout to option holders at expiry
/// (Σ call OI·max(S−K,0) + Σ put OI·max(K−S,0) over candidate strikes).
/// Returns `(strike, total_payout_at_strike)`; `None` without open interest.
pub fn max_pain_strike(expiry: &OptionExpiry) -> Option<(f64, f64)> {
    let mut strikes: Vec<f64> = expiry
        .calls
        .iter()
        .chain(expiry.puts.iter())
        .map(|c| c.strike)
        .filter(|k| k.is_finite() && *k > 0.0)
        .collect();
    strikes.sort_by(|a, b| a.total_cmp(b));
    strikes.dedup_by(|a, b| (*a - *b).abs() < 1e-9);
    if strikes.is_empty() {
        return None;
    }
    let has_oi = expiry
        .calls
        .iter()
        .chain(expiry.puts.iter())
        .any(|c| c.open_interest > 0.0);
    if !has_oi {
        return None;
    }
    let mut best: Option<(f64, f64)> = None;
    for &s in &strikes {
        let call_pay: f64 = expiry
            .calls
            .iter()
            .map(|c| c.open_interest * (s - c.strike).max(0.0))
            .sum();
        let put_pay: f64 = expiry
            .puts
            .iter()
            .map(|c| c.open_interest * (c.strike - s).max(0.0))
            .sum();
        let total = call_pay + put_pay;
        if best.map(|(_, b)| total < b).unwrap_or(true) {
            best = Some((s, total));
        }
    }
    best
}

/// Max-pain strike per cached expiration: `(expiration, strike)`.
pub fn max_pain_by_expiration(chain: &OptionsChainSnapshot) -> Vec<(String, f64)> {
    chain
        .expirations
        .iter()
        .filter_map(|exp| max_pain_strike(exp).map(|(k, _)| (exp.expiration.clone(), k)))
        .collect()
}

#[cfg(test)]
mod max_pain_tests {
    use super::*;

    fn contract(kind: &str, strike: f64, oi: f64) -> OptionContract {
        OptionContract {
            option_type: kind.into(),
            strike,
            open_interest: oi,
            ..Default::default()
        }
    }

    #[test]
    fn max_pain_minimizes_holder_payout() {
        // Heavy call OI at 100 and put OI at 100 pins pain at 100; a stray
        // 120 call and 80 put don't move it.
        let expiry = OptionExpiry {
            expiration: "2026-08-21".into(),
            days_to_expiry: 48,
            calls: vec![
                contract("CALL", 100.0, 1000.0),
                contract("CALL", 120.0, 50.0),
            ],
            puts: vec![contract("PUT", 100.0, 1000.0), contract("PUT", 80.0, 50.0)],
        };
        let (strike, _) = max_pain_strike(&expiry).unwrap();
        assert!((strike - 100.0).abs() < 1e-9);

        // No OI → no answer.
        let empty = OptionExpiry {
            calls: vec![contract("CALL", 100.0, 0.0)],
            puts: vec![],
            ..Default::default()
        };
        assert!(max_pain_strike(&empty).is_none());
    }
}

/// IVOL — one ATM IV observation over time (52-week history bucket).
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct IvolObservation {
    pub date: String, // YYYY-MM-DD
    pub atm_iv_pct: f64,
}

/// IVOL — implied-volatility rank and percentile snapshot for a symbol.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct IvolSnapshot {
    pub symbol: String,
    pub as_of: String,
    pub current_atm_iv_pct: f64,
    pub iv_52w_low_pct: f64,
    pub iv_52w_high_pct: f64,
    pub iv_rank: f64,       // 0..100: (current - low) / (high - low) × 100
    pub iv_percentile: f64, // 0..100: % of days at or below current
    pub observation_count: usize,
    pub history: Vec<IvolObservation>,
    pub note: String,
}
