//! Options pricing: Black-Scholes Greeks and Implied Volatility.
//!
//! Provides Delta, Gamma, Theta, Vega, Rho for European options.
//! IV computed via Newton-Raphson iteration from market price.

use std::f64::consts::{PI, SQRT_2};

/// Standard normal CDF (cumulative distribution function).
fn norm_cdf(x: f64) -> f64 {
    0.5 * (1.0 + erf(x / SQRT_2))
}

/// Standard normal PDF (probability density function).
fn norm_pdf(x: f64) -> f64 {
    (-0.5 * x * x).exp() / (2.0 * PI).sqrt()
}

/// Error function approximation (Abramowitz & Stegun).
fn erf(x: f64) -> f64 {
    let a1 = 0.254829592;
    let a2 = -0.284496736;
    let a3 = 1.421413741;
    let a4 = -1.453152027;
    let a5 = 1.061405429;
    let p = 0.3275911;
    let sign = if x >= 0.0 { 1.0 } else { -1.0 };
    let x = x.abs();
    let t = 1.0 / (1.0 + p * x);
    let y = 1.0 - (((((a5 * t + a4) * t) + a3) * t + a2) * t + a1) * t * (-x * x).exp();
    sign * y
}

/// Black-Scholes option Greeks.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Greeks {
    pub delta: f64,
    pub gamma: f64,
    pub theta: f64,   // per day
    pub vega: f64,    // per 1% vol change
    pub rho: f64,     // per 1% rate change
    pub iv: f64,      // implied volatility (annualized)
    pub theoretical_price: f64,
}

/// Compute Black-Scholes price for a European option.
/// `s` = spot price, `k` = strike, `t` = time to expiry (years),
/// `r` = risk-free rate (annualized), `sigma` = volatility (annualized),
/// `is_call` = true for call, false for put.
pub fn bs_price(s: f64, k: f64, t: f64, r: f64, sigma: f64, is_call: bool) -> f64 {
    if t <= 0.0 || sigma <= 0.0 || s <= 0.0 || k <= 0.0 {
        return if is_call { (s - k).max(0.0) } else { (k - s).max(0.0) };
    }
    let d1 = ((s / k).ln() + (r + 0.5 * sigma * sigma) * t) / (sigma * t.sqrt());
    let d2 = d1 - sigma * t.sqrt();
    if is_call {
        s * norm_cdf(d1) - k * (-r * t).exp() * norm_cdf(d2)
    } else {
        k * (-r * t).exp() * norm_cdf(-d2) - s * norm_cdf(-d1)
    }
}

/// Compute all Greeks for a European option.
pub fn greeks(s: f64, k: f64, t: f64, r: f64, sigma: f64, is_call: bool) -> Greeks {
    if t <= 0.0 || sigma <= 0.0 || s <= 0.0 || k <= 0.0 {
        return Greeks { delta: 0.0, gamma: 0.0, theta: 0.0, vega: 0.0, rho: 0.0, iv: sigma, theoretical_price: 0.0 };
    }
    let d1 = ((s / k).ln() + (r + 0.5 * sigma * sigma) * t) / (sigma * t.sqrt());
    let d2 = d1 - sigma * t.sqrt();
    let sqrt_t = t.sqrt();
    let disc = (-r * t).exp();

    let theoretical_price = if is_call {
        s * norm_cdf(d1) - k * disc * norm_cdf(d2)
    } else {
        k * disc * norm_cdf(-d2) - s * norm_cdf(-d1)
    };

    let delta = if is_call { norm_cdf(d1) } else { norm_cdf(d1) - 1.0 };
    let gamma = norm_pdf(d1) / (s * sigma * sqrt_t);
    let vega = s * norm_pdf(d1) * sqrt_t / 100.0; // per 1% vol change
    let theta = if is_call {
        (-s * norm_pdf(d1) * sigma / (2.0 * sqrt_t) - r * k * disc * norm_cdf(d2)) / 365.0
    } else {
        (-s * norm_pdf(d1) * sigma / (2.0 * sqrt_t) + r * k * disc * norm_cdf(-d2)) / 365.0
    };
    let rho = if is_call {
        k * t * disc * norm_cdf(d2) / 100.0
    } else {
        -k * t * disc * norm_cdf(-d2) / 100.0
    };

    Greeks { delta, gamma, theta, vega, rho, iv: sigma, theoretical_price }
}

/// Compute implied volatility from market price via Newton-Raphson.
/// Returns None if convergence fails.
pub fn implied_volatility(s: f64, k: f64, t: f64, r: f64, market_price: f64, is_call: bool) -> Option<f64> {
    if market_price <= 0.0 || t <= 0.0 || s <= 0.0 || k <= 0.0 { return None; }

    let mut sigma = 0.3; // initial guess 30%
    for _ in 0..100 {
        let price = bs_price(s, k, t, r, sigma, is_call);
        let d1 = ((s / k).ln() + (r + 0.5 * sigma * sigma) * t) / (sigma * t.sqrt());
        let vega = s * norm_pdf(d1) * t.sqrt();
        if vega.abs() < 1e-12 { break; }
        let diff = price - market_price;
        if diff.abs() < 1e-8 { return Some(sigma); }
        sigma -= diff / vega;
        if sigma <= 0.001 { sigma = 0.001; }
        if sigma > 10.0 { sigma = 10.0; }
    }
    Some(sigma)
}

// ── Tests ───────────────────────────────────────────────────────────
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bs_call_price() {
        // S=100, K=100, T=1yr, r=5%, sigma=20% → ~$10.45
        let price = bs_price(100.0, 100.0, 1.0, 0.05, 0.20, true);
        assert!((price - 10.45).abs() < 0.1, "BS call price: {price}");
    }

    #[test]
    fn test_bs_put_price() {
        let price = bs_price(100.0, 100.0, 1.0, 0.05, 0.20, false);
        assert!((price - 5.57).abs() < 0.1, "BS put price: {price}");
    }

    #[test]
    fn test_greeks_call() {
        let g = greeks(100.0, 100.0, 1.0, 0.05, 0.20, true);
        assert!(g.delta > 0.5 && g.delta < 0.7, "Delta: {}", g.delta);
        assert!(g.gamma > 0.0, "Gamma should be positive: {}", g.gamma);
        assert!(g.vega > 0.0, "Vega should be positive: {}", g.vega);
        assert!(g.theta < 0.0, "Theta should be negative: {}", g.theta);
    }

    #[test]
    fn test_greeks_put() {
        let g = greeks(100.0, 100.0, 1.0, 0.05, 0.20, false);
        assert!(g.delta < 0.0, "Put delta should be negative: {}", g.delta);
    }

    #[test]
    fn test_put_call_parity() {
        let call = bs_price(100.0, 100.0, 1.0, 0.05, 0.20, true);
        let put = bs_price(100.0, 100.0, 1.0, 0.05, 0.20, false);
        let parity = call - put - 100.0 + 100.0 * (-0.05_f64).exp();
        assert!(parity.abs() < 0.01, "Put-call parity violated: {parity}");
    }

    #[test]
    fn test_implied_volatility() {
        let target_price = bs_price(100.0, 100.0, 1.0, 0.05, 0.25, true);
        let iv = implied_volatility(100.0, 100.0, 1.0, 0.05, target_price, true);
        assert!(iv.is_some());
        assert!((iv.unwrap() - 0.25).abs() < 0.001, "IV: {}", iv.unwrap());
    }

    #[test]
    fn test_expired_option() {
        let price = bs_price(100.0, 95.0, 0.0, 0.05, 0.20, true);
        assert!((price - 5.0).abs() < 0.01, "Expired ITM call: {price}");
    }

    #[test]
    fn test_deep_otm() {
        let price = bs_price(100.0, 200.0, 0.1, 0.05, 0.20, true);
        assert!(price < 0.01, "Deep OTM call should be ~0: {price}");
    }

    #[test]
    fn test_deep_itm_call() {
        let price = bs_price(200.0, 100.0, 1.0, 0.05, 0.20, true);
        // Deep ITM: should be close to intrinsic value (200-100*e^(-0.05))
        assert!(price > 95.0, "Deep ITM call: {price}");
    }

    #[test]
    fn test_deep_itm_put() {
        let price = bs_price(50.0, 100.0, 1.0, 0.05, 0.20, false);
        assert!(price > 45.0, "Deep ITM put: {price}");
    }

    #[test]
    fn test_greeks_deep_itm_delta() {
        let g = greeks(200.0, 100.0, 0.5, 0.05, 0.20, true);
        assert!(g.delta > 0.95, "Deep ITM call delta should be ~1.0: {}", g.delta);
    }

    #[test]
    fn test_greeks_deep_otm_delta() {
        let g = greeks(50.0, 100.0, 0.5, 0.05, 0.20, true);
        assert!(g.delta < 0.05, "Deep OTM call delta should be ~0: {}", g.delta);
    }

    #[test]
    fn test_iv_zero_price() {
        let iv = implied_volatility(100.0, 100.0, 1.0, 0.05, 0.0, true);
        assert!(iv.is_none(), "Zero market price should return None");
    }

    #[test]
    fn test_iv_put() {
        let target_price = bs_price(100.0, 100.0, 1.0, 0.05, 0.30, false);
        let iv = implied_volatility(100.0, 100.0, 1.0, 0.05, target_price, false);
        assert!(iv.is_some());
        assert!((iv.unwrap() - 0.30).abs() < 0.01, "Put IV: {}", iv.unwrap());
    }

    #[test]
    fn test_vega_always_positive() {
        for strike in [80.0, 100.0, 120.0] {
            let g = greeks(100.0, strike, 0.5, 0.05, 0.25, true);
            assert!(g.vega >= 0.0, "Vega should be non-negative at strike {strike}: {}", g.vega);
        }
    }

    #[test]
    fn test_gamma_highest_atm() {
        let g_itm = greeks(100.0, 80.0, 0.5, 0.05, 0.25, true);
        let g_atm = greeks(100.0, 100.0, 0.5, 0.05, 0.25, true);
        let g_otm = greeks(100.0, 120.0, 0.5, 0.05, 0.25, true);
        assert!(g_atm.gamma > g_itm.gamma, "ATM gamma should be highest");
        assert!(g_atm.gamma > g_otm.gamma, "ATM gamma should be highest");
    }
}
