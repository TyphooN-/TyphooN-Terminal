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
    max_pain_strike_optimized(expiry)
}

fn max_pain_candidate_strikes(expiry: &OptionExpiry) -> Vec<f64> {
    let mut strikes: Vec<f64> = expiry
        .calls
        .iter()
        .chain(expiry.puts.iter())
        .map(|c| c.strike)
        .filter(|k| k.is_finite() && *k > 0.0)
        .collect();
    strikes.sort_by(|a, b| a.total_cmp(b));
    strikes.dedup_by(|a, b| (*a - *b).abs() < 1e-9);
    strikes
}

fn has_positive_open_interest(expiry: &OptionExpiry) -> bool {
    expiry
        .calls
        .iter()
        .chain(expiry.puts.iter())
        .any(|contract| contract.open_interest > 0.0)
}

fn max_pain_payout_at_strike(expiry: &OptionExpiry, strike: f64) -> f64 {
    let call_pay: f64 = expiry
        .calls
        .iter()
        .map(|contract| contract.open_interest * (strike - contract.strike).max(0.0))
        .sum();
    let put_pay: f64 = expiry
        .puts
        .iter()
        .map(|contract| contract.open_interest * (contract.strike - strike).max(0.0))
        .sum();
    call_pay + put_pay
}

fn weighted_subtraction_is_ill_conditioned(result: f64, left: f64, right: f64) -> bool {
    let scale = left.abs() + right.abs();
    scale > 0.0 && result.abs() <= scale * f64::EPSILON * 16.0
}

fn max_pain_strike_brute_force(expiry: &OptionExpiry) -> Option<(f64, f64)> {
    let strikes = max_pain_candidate_strikes(expiry);
    if strikes.is_empty() || !has_positive_open_interest(expiry) {
        return None;
    }
    let mut best: Option<(f64, f64)> = None;
    for &s in &strikes {
        let total = max_pain_payout_at_strike(expiry, s);
        if best.map(|(_, b)| total < b).unwrap_or(true) {
            best = Some((s, total));
        }
    }
    best
}

fn max_pain_strike_optimized(expiry: &OptionExpiry) -> Option<(f64, f64)> {
    let strikes = max_pain_candidate_strikes(expiry);
    if strikes.is_empty() || !has_positive_open_interest(expiry) {
        return None;
    }

    // The sweep relies on finite arithmetic. Preserve the legacy calculation for
    // malformed provider values and for any aggregate overflow below.
    if expiry
        .calls
        .iter()
        .chain(expiry.puts.iter())
        .any(|contract| {
            !contract.strike.is_finite()
                || !contract.open_interest.is_finite()
                || contract.open_interest < 0.0
        })
    {
        return max_pain_strike_brute_force(expiry);
    }

    let mut calls: Vec<&OptionContract> = expiry.calls.iter().collect();
    let mut puts: Vec<&OptionContract> = expiry.puts.iter().collect();
    calls.sort_by(|a, b| a.strike.total_cmp(&b.strike));
    puts.sort_by(|a, b| a.strike.total_cmp(&b.strike));

    let mut call_prefix_oi = vec![0.0; calls.len() + 1];
    let mut call_prefix_weighted_strike = vec![0.0; calls.len() + 1];
    for (index, contract) in calls.iter().enumerate() {
        call_prefix_oi[index + 1] = call_prefix_oi[index] + contract.open_interest;
        call_prefix_weighted_strike[index + 1] =
            call_prefix_weighted_strike[index] + contract.open_interest * contract.strike;
    }
    let mut put_suffix_oi = vec![0.0; puts.len() + 1];
    let mut put_suffix_weighted_strike = vec![0.0; puts.len() + 1];
    for index in (0..puts.len()).rev() {
        let contract = puts[index];
        put_suffix_oi[index] = put_suffix_oi[index + 1] + contract.open_interest;
        put_suffix_weighted_strike[index] =
            put_suffix_weighted_strike[index + 1] + contract.open_interest * contract.strike;
    }
    if call_prefix_oi.iter().any(|value| !value.is_finite())
        || call_prefix_weighted_strike
            .iter()
            .any(|value| !value.is_finite())
        || put_suffix_oi.iter().any(|value| !value.is_finite())
        || put_suffix_weighted_strike
            .iter()
            .any(|value| !value.is_finite())
    {
        return max_pain_strike_brute_force(expiry);
    }

    let mut call_index = 0;
    let mut put_index = 0;
    let mut best: Option<(f64, f64, f64)> = None;
    let rounding_factor = f64::EPSILON * (calls.len() + puts.len() + 8) as f64 * 8.0;

    for strike in strikes {
        while call_index < calls.len() && calls[call_index].strike < strike {
            call_index += 1;
        }
        while put_index < puts.len() && puts[put_index].strike <= strike {
            put_index += 1;
        }

        let call_left = strike * call_prefix_oi[call_index];
        let call_right = call_prefix_weighted_strike[call_index];
        let call_pay = call_left - call_right;
        let put_left = put_suffix_weighted_strike[put_index];
        let put_right = strike * put_suffix_oi[put_index];
        let put_pay = put_left - put_right;
        if call_pay < 0.0
            || put_pay < 0.0
            || weighted_subtraction_is_ill_conditioned(call_pay, call_left, call_right)
            || weighted_subtraction_is_ill_conditioned(put_pay, put_left, put_right)
        {
            return max_pain_strike_brute_force(expiry);
        }
        let total = call_pay + put_pay;
        if !total.is_finite() {
            return max_pain_strike_brute_force(expiry);
        }
        let error_bound = rounding_factor
            * (call_left.abs() + call_right.abs() + put_left.abs() + put_right.abs());
        if let Some((_, best_payout, best_error_bound)) = best {
            if (total - best_payout).abs() <= error_bound + best_error_bound {
                return max_pain_strike_brute_force(expiry);
            }
            if total < best_payout {
                best = Some((strike, total, error_bound));
            }
        } else {
            best = Some((strike, total, error_bound));
        }
    }
    best.map(|(strike, _, _)| (strike, max_pain_payout_at_strike(expiry, strike)))
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

    fn brute_force_max_pain(expiry: &OptionExpiry) -> Option<(f64, f64)> {
        let mut strikes: Vec<f64> = expiry
            .calls
            .iter()
            .chain(expiry.puts.iter())
            .map(|contract| contract.strike)
            .filter(|strike| strike.is_finite() && *strike > 0.0)
            .collect();
        strikes.sort_by(|a, b| a.total_cmp(b));
        strikes.dedup_by(|a, b| (*a - *b).abs() < 1e-9);
        if strikes.is_empty()
            || !expiry
                .calls
                .iter()
                .chain(expiry.puts.iter())
                .any(|contract| contract.open_interest > 0.0)
        {
            return None;
        }
        strikes
            .into_iter()
            .map(|strike| {
                let call_pay: f64 = expiry
                    .calls
                    .iter()
                    .map(|contract| contract.open_interest * (strike - contract.strike).max(0.0))
                    .sum();
                let put_pay: f64 = expiry
                    .puts
                    .iter()
                    .map(|contract| contract.open_interest * (contract.strike - strike).max(0.0))
                    .sum();
                (strike, call_pay + put_pay)
            })
            .reduce(|best, candidate| {
                if candidate.1 < best.1 {
                    candidate
                } else {
                    best
                }
            })
    }

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

    #[test]
    fn optimized_max_pain_matches_brute_force_oracle() {
        let mut state = 0x4d59_5df4_d0f3_3173_u64;
        let mut next = || {
            state = state
                .wrapping_mul(6_364_136_223_846_793_005)
                .wrapping_add(1_442_695_040_888_963_407);
            state
        };

        for case in 0..256 {
            let call_count = (next() % 24 + 1) as usize;
            let put_count = (next() % 24 + 1) as usize;
            let mut calls = Vec::with_capacity(call_count);
            let mut puts = Vec::with_capacity(put_count);
            for index in 0..call_count {
                let base = 50.0 + (next() % 41) as f64 * 2.5;
                let strike = if index % 7 == 0 { base + 5e-10 } else { base };
                let oi = if (case + index) % 17 == 0 {
                    1e16
                } else {
                    (next() % 10_000) as f64 / 10.0
                };
                calls.push(contract("CALL", strike, oi));
            }
            for index in 0..put_count {
                let base = 50.0 + (next() % 41) as f64 * 2.5;
                let strike = if index % 9 == 0 { base + 5e-10 } else { base };
                let oi = if (case + index) % 19 == 0 {
                    1e16
                } else {
                    (next() % 10_000) as f64 / 10.0
                };
                puts.push(contract("PUT", strike, oi));
            }
            let expiry = OptionExpiry {
                calls,
                puts,
                ..Default::default()
            };

            let expected = brute_force_max_pain(&expiry).unwrap();
            let actual = max_pain_strike_optimized(&expiry).unwrap();
            assert_eq!(
                actual.0, expected.0,
                "candidate mismatch in case {case}: {expiry:#?}; expected {expected:?}, got {actual:?}"
            );
            let tolerance = expected.1.abs().max(1.0) * 1e-10;
            assert!(
                (actual.1 - expected.1).abs() <= tolerance,
                "payout mismatch in case {case}: expected {}, got {}",
                expected.1,
                actual.1
            );
        }
    }

    #[test]
    fn max_pain_preserves_edge_case_provider_semantics() {
        let cases = [
            OptionExpiry {
                calls: vec![contract("CALL", -10.0, 25.0), contract("CALL", 100.0, 50.0)],
                puts: vec![contract("PUT", 110.0, 40.0)],
                ..Default::default()
            },
            OptionExpiry {
                calls: vec![contract("CALL", 90.0, -5.0), contract("CALL", 100.0, 50.0)],
                puts: vec![contract("PUT", 110.0, 40.0)],
                ..Default::default()
            },
            OptionExpiry {
                calls: vec![
                    contract("CALL", f64::NAN, 10.0),
                    contract("CALL", 100.0, 50.0),
                ],
                puts: vec![contract("PUT", 110.0, 40.0)],
                ..Default::default()
            },
            OptionExpiry {
                calls: vec![contract("CALL", 100.0, f64::NAN)],
                puts: vec![contract("PUT", 110.0, 40.0)],
                ..Default::default()
            },
        ];

        for expiry in cases {
            let expected = brute_force_max_pain(&expiry).unwrap();
            let actual = max_pain_strike(&expiry).unwrap();
            assert_eq!(actual.0, expected.0);
            if expected.1.is_nan() {
                assert!(actual.1.is_nan());
            } else {
                assert_eq!(actual.1, expected.1);
            }
        }

        let zero_oi = OptionExpiry {
            calls: vec![contract("CALL", 100.0, 0.0)],
            puts: vec![contract("PUT", 110.0, 0.0)],
            ..Default::default()
        };
        assert!(max_pain_strike(&zero_oi).is_none());
    }

    #[test]
    fn max_pain_preserves_small_suffix_after_huge_expired_put() {
        let expiry = OptionExpiry {
            calls: vec![contract("CALL", 1.0, 2.0)],
            puts: vec![contract("PUT", 100.0, 1e16), contract("PUT", 110.0, 3.0)],
            ..Default::default()
        };

        let expected = brute_force_max_pain(&expiry).unwrap();
        assert_eq!(expected, (110.0, 218.0));
        assert_eq!(max_pain_strike(&expiry), Some(expected));
    }

    #[test]
    fn max_pain_falls_back_when_weighted_sums_cannot_rank_close_strikes() {
        let expiry = OptionExpiry {
            calls: vec![
                contract("CALL", 99.99999998, 1.0),
                contract("CALL", 99.99999998, 3.0),
                contract("CALL", 99.99999993, 0.0),
                contract("CALL", 99.99999993, 1.0),
                contract("CALL", 99.99999993, 1.0),
                contract("CALL", 99.99999998, 0.0),
                contract("CALL", 99.99999993, 1e16),
            ],
            puts: vec![
                contract("PUT", 99.99999993, 1.0),
                contract("PUT", 99.99999992, 1000.0),
                contract("PUT", 99.99999993, 0.0),
                contract("PUT", 99.99999998, 1e16),
                contract("PUT", 99.99999993, 3.0),
            ],
            ..Default::default()
        };

        let expected = brute_force_max_pain(&expiry).unwrap();
        assert_eq!(expected.0, 99.99999993);
        assert_eq!(max_pain_strike(&expiry), Some(expected));
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
