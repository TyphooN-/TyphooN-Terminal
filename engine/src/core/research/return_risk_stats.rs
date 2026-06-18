//! Return-distribution and risk-statistical research snapshot computations.

use super::*;

mod autocorr_regime;
mod distribution_shape;
mod downside_efficiency;
mod drawdown_liquidity_normality;
mod drawup_gap_range;
mod seasonality_spread;
mod volatility_estimators;
mod performance_runs_tests;
mod significance_stationarity;
mod tail_risk_diagnostics;
mod entropy_dependence;
mod upside_drawdown_risk;
mod entropy_stationarity;
mod robust_quantile_volatility;
mod normality_liquidity_tail;
pub use autocorr_regime::*;
use autocorr_regime::acf_at_lag;
pub use distribution_shape::*;
pub use downside_efficiency::*;
pub use drawdown_liquidity_normality::*;
pub use drawup_gap_range::*;
pub use seasonality_spread::*;
pub use volatility_estimators::*;
pub use performance_runs_tests::*;
pub use significance_stationarity::*;
pub use tail_risk_diagnostics::*;
pub use entropy_dependence::*;
pub use upside_drawdown_risk::*;
pub use entropy_stationarity::*;
pub use robust_quantile_volatility::*;
pub use normality_liquidity_tail::*;

// Shared helpers for return-distribution and risk-statistical compute modules.

/// Shared helper: collect trailing 253 bars sorted oldest-first and
/// compute log returns. Returns (sorted_bars, log_returns).
pub(crate) fn trailing_log_returns(
    bars: &[HistoricalPriceRow],
) -> (Vec<&HistoricalPriceRow>, Vec<f64>) {
    let mut sorted: Vec<&HistoricalPriceRow> = bars.iter().collect();
    sorted.sort_by(|a, b| a.date.cmp(&b.date));
    let window: Vec<&HistoricalPriceRow> = sorted.iter().rev().take(253).rev().copied().collect();
    let mut log_rets: Vec<f64> = Vec::with_capacity(window.len());
    for w in window.windows(2) {
        let prev = w[0].close;
        let curr = w[1].close;
        if prev > 0.0 && curr > 0.0 {
            log_rets.push((curr / prev).ln());
        }
    }
    (window, log_rets)
}

// Higuchi, Pickands, kappa, Lyapunov, and rank-autocorrelation computes

/// HIGUCHI compute: Higuchi 1988 fractal dimension.
pub fn compute_higuchi_snapshot(
    symbol: &str,
    as_of: &str,
    bars: &[HistoricalPriceRow],
) -> HiguchiSnapshot {
    let sym = symbol.to_uppercase();
    let (_, log_rets) = trailing_log_returns(bars);
    if log_rets.len() < 100 {
        return HiguchiSnapshot {
            symbol: sym,
            as_of: as_of.into(),
            higuchi_label: "INSUFFICIENT_DATA".into(),
            note: format!("need ≥100 returns, got {}", log_rets.len()),
            ..Default::default()
        };
    }
    let n = log_rets.len();
    // cumulative sum so that "fluctuation" acts on a walk, per Higuchi convention
    let mut x = Vec::with_capacity(n);
    let mut s = 0.0f64;
    for &r in &log_rets {
        s += r;
        x.push(s);
    }
    let k_max = 10usize;
    let mut log_k: Vec<f64> = Vec::new();
    let mut log_l: Vec<f64> = Vec::new();
    for k in 1..=k_max {
        let mut lk_sum = 0.0f64;
        let mut count = 0usize;
        for m in 0..k {
            // indices m, m+k, m+2k, ...
            let max_i = (n - 1 - m) / k;
            if max_i < 1 {
                continue;
            }
            let mut l_m = 0.0f64;
            for i in 1..=max_i {
                l_m += (x[m + i * k] - x[m + (i - 1) * k]).abs();
            }
            let norm = ((n - 1) as f64) / ((max_i * k) as f64);
            l_m = l_m * norm / (k as f64);
            lk_sum += l_m;
            count += 1;
        }
        if count == 0 {
            continue;
        }
        let l_avg = lk_sum / count as f64;
        if l_avg > 0.0 {
            log_k.push((1.0 / k as f64).ln());
            log_l.push(l_avg.ln());
        }
    }
    if log_k.len() < 3 {
        return HiguchiSnapshot {
            symbol: sym,
            as_of: as_of.into(),
            higuchi_label: "INSUFFICIENT_DATA".into(),
            note: "insufficient log-k points".into(),
            ..Default::default()
        };
    }
    // Linear regression log_l = fd · log_k + c  (note: we want slope w.r.t. ln(1/k))
    let m = log_k.len() as f64;
    let mx: f64 = log_k.iter().sum::<f64>() / m;
    let my: f64 = log_l.iter().sum::<f64>() / m;
    let mut sxx = 0.0f64;
    let mut sxy = 0.0f64;
    let mut syy = 0.0f64;
    for i in 0..log_k.len() {
        let dx = log_k[i] - mx;
        let dy = log_l[i] - my;
        sxx += dx * dx;
        sxy += dx * dy;
        syy += dy * dy;
    }
    if sxx < f64::EPSILON {
        return HiguchiSnapshot {
            symbol: sym,
            as_of: as_of.into(),
            higuchi_label: "INSUFFICIENT_DATA".into(),
            note: "no variation in log k".into(),
            ..Default::default()
        };
    }
    let fd = sxy / sxx; // slope
    let r2 = if syy > f64::EPSILON {
        (sxy * sxy) / (sxx * syy)
    } else {
        0.0
    };
    let label = if fd < 1.1 {
        "SMOOTH"
    } else if fd < 1.4 {
        "PERSISTENT"
    } else if fd < 1.6 {
        "RANDOM"
    } else {
        "ROUGH"
    };
    HiguchiSnapshot {
        symbol: sym,
        as_of: as_of.into(),
        bars_used: n,
        k_max,
        fractal_dim: fd,
        r_squared: r2,
        log_k_count: log_k.len(),
        higuchi_label: label.into(),
        note: String::new(),
    }
}

/// PICKANDS compute: Pickands 1975 tail-index estimator.
pub fn compute_pickands_snapshot(
    symbol: &str,
    as_of: &str,
    bars: &[HistoricalPriceRow],
) -> PickandsSnapshot {
    let sym = symbol.to_uppercase();
    let (_, log_rets) = trailing_log_returns(bars);
    if log_rets.len() < 80 {
        return PickandsSnapshot {
            symbol: sym,
            as_of: as_of.into(),
            pickands_label: "INSUFFICIENT_DATA".into(),
            note: format!("need ≥80 returns, got {}", log_rets.len()),
            ..Default::default()
        };
    }
    let n = log_rets.len();
    let mut abs_r: Vec<f64> = log_rets.iter().map(|r| r.abs()).collect();
    // sort descending so index i=0 is the largest
    abs_r.sort_by(|a, b| b.partial_cmp(a).unwrap_or(std::cmp::Ordering::Equal));
    // Pickands requires at least 4k+1 samples. Use k = n/16 ⇒ 4k < n.
    let k = (n / 16).max(5);
    if 4 * k >= n {
        return PickandsSnapshot {
            symbol: sym,
            as_of: as_of.into(),
            pickands_label: "INSUFFICIENT_DATA".into(),
            note: format!("4k={} ≥ n={}", 4 * k, n),
            ..Default::default()
        };
    }
    let x_k = abs_r[k - 1];
    let x_2k = abs_r[2 * k - 1];
    let x_4k = abs_r[4 * k - 1];
    let num = x_k - x_2k;
    let den = x_2k - x_4k;
    if den.abs() < f64::EPSILON || num.abs() < f64::EPSILON {
        return PickandsSnapshot {
            symbol: sym,
            as_of: as_of.into(),
            pickands_label: "INSUFFICIENT_DATA".into(),
            note: "degenerate order-stat differences".into(),
            ..Default::default()
        };
    }
    let ratio = num / den;
    if ratio <= 0.0 {
        return PickandsSnapshot {
            symbol: sym,
            as_of: as_of.into(),
            pickands_label: "INSUFFICIENT_DATA".into(),
            note: format!("ratio {} ≤ 0", ratio),
            ..Default::default()
        };
    }
    let gamma_hat = ratio.ln() / std::f64::consts::LN_2;
    let tail_index = if gamma_hat.abs() < f64::EPSILON {
        f64::INFINITY
    } else {
        1.0 / gamma_hat
    };
    let label = if gamma_hat > 0.5 {
        "FRECHET_HEAVY"
    } else if gamma_hat > 0.1 {
        "FRECHET_MODERATE"
    } else if gamma_hat > -0.1 {
        "GUMBEL_EXPONENTIAL"
    } else {
        "WEIBULL_BOUNDED"
    };
    PickandsSnapshot {
        symbol: sym,
        as_of: as_of.into(),
        bars_used: n,
        k_index: k,
        gamma_hat,
        tail_index,
        x_k,
        x_2k,
        x_4k,
        pickands_label: label.into(),
        note: String::new(),
    }
}

/// KAPPA3 compute: Kaplan-Knowles 2004 Kappa-3 ratio.
pub fn compute_kappa3_snapshot(
    symbol: &str,
    as_of: &str,
    bars: &[HistoricalPriceRow],
) -> Kappa3Snapshot {
    let sym = symbol.to_uppercase();
    let (_, log_rets) = trailing_log_returns(bars);
    if log_rets.len() < 30 {
        return Kappa3Snapshot {
            symbol: sym,
            as_of: as_of.into(),
            kappa3_label: "INSUFFICIENT_DATA".into(),
            note: format!("need ≥30 returns, got {}", log_rets.len()),
            ..Default::default()
        };
    }
    let n = log_rets.len();
    let nf = n as f64;
    let mar = 0.0_f64;
    let mean = log_rets.iter().sum::<f64>() / nf;
    // Annualise with ×252 for excess-mean and ×√252 for lpm roots
    let excess_mean_ann = (mean - mar) * 252.0;
    let mut lpm2 = 0.0f64;
    let mut lpm3 = 0.0f64;
    for &r in &log_rets {
        let d = (mar - r).max(0.0);
        lpm2 += d * d;
        lpm3 += d * d * d;
    }
    lpm2 /= nf;
    lpm3 /= nf;
    if lpm2 < f64::EPSILON || lpm3 < f64::EPSILON {
        return Kappa3Snapshot {
            symbol: sym,
            as_of: as_of.into(),
            kappa3_label: "INSUFFICIENT_DATA".into(),
            note: "zero lower partial moment".into(),
            ..Default::default()
        };
    }
    let lpm3_root = lpm3.powf(1.0 / 3.0);
    // Annualise the downside risk: ×252^(1/3) for cube-root LPM, ×√252 for squared LPM
    let lpm3_root_ann = lpm3_root * (252.0_f64).powf(1.0 / 3.0);
    let lpm2_root_ann = lpm2.sqrt() * (252.0_f64).sqrt();
    let kappa3 = excess_mean_ann / lpm3_root_ann;
    let sortino = excess_mean_ann / lpm2_root_ann;
    let label = if kappa3 > 1.0 {
        "STRONG"
    } else if kappa3 > 0.0 {
        "POSITIVE"
    } else if kappa3 > -0.5 {
        "NEUTRAL"
    } else {
        "NEGATIVE"
    };
    Kappa3Snapshot {
        symbol: sym,
        as_of: as_of.into(),
        bars_used: n,
        mar,
        excess_mean: excess_mean_ann,
        lpm3,
        lpm3_root: lpm3_root_ann,
        kappa3,
        sortino_compare: sortino,
        kappa3_label: label.into(),
        note: String::new(),
    }
}

/// LYAPUNOV compute: Rosenstein et al. 1993 largest Lyapunov exponent.
pub fn compute_lyapunov_snapshot(
    symbol: &str,
    as_of: &str,
    bars: &[HistoricalPriceRow],
) -> LyapunovSnapshot {
    let sym = symbol.to_uppercase();
    let (_, log_rets) = trailing_log_returns(bars);
    if log_rets.len() < 100 {
        return LyapunovSnapshot {
            symbol: sym,
            as_of: as_of.into(),
            lyapunov_label: "INSUFFICIENT_DATA".into(),
            note: format!("need ≥100 returns, got {}", log_rets.len()),
            ..Default::default()
        };
    }
    let n = log_rets.len();
    let m = 3usize;
    let tau = 1usize;
    let n_vec = n - (m - 1) * tau;
    if n_vec < 30 {
        return LyapunovSnapshot {
            symbol: sym,
            as_of: as_of.into(),
            lyapunov_label: "INSUFFICIENT_DATA".into(),
            note: "too few embedding vectors".into(),
            ..Default::default()
        };
    }
    // Build embedded vectors
    let mut vecs: Vec<[f64; 3]> = Vec::with_capacity(n_vec);
    for i in 0..n_vec {
        vecs.push([log_rets[i], log_rets[i + tau], log_rets[i + 2 * tau]]);
    }
    // For each reference point, find nearest neighbour (excluding Theiler window)
    let theiler = 10usize;
    let max_steps = 20usize;
    let mut log_d_sum = vec![0.0f64; max_steps];
    let mut log_d_cnt = vec![0usize; max_steps];
    for i in 0..vecs.len() {
        let mut best_j: Option<usize> = None;
        let mut best_d = f64::INFINITY;
        for j in 0..vecs.len() {
            if (j as i64 - i as i64).unsigned_abs() as usize <= theiler {
                continue;
            }
            let dx = vecs[i][0] - vecs[j][0];
            let dy = vecs[i][1] - vecs[j][1];
            let dz = vecs[i][2] - vecs[j][2];
            let d2 = dx * dx + dy * dy + dz * dz;
            if d2 < best_d {
                best_d = d2;
                best_j = Some(j);
            }
        }
        if let Some(j) = best_j {
            if best_d <= f64::EPSILON {
                continue;
            }
            for step in 0..max_steps {
                let ii = i + step;
                let jj = j + step;
                if ii >= vecs.len() || jj >= vecs.len() {
                    break;
                }
                let dx = vecs[ii][0] - vecs[jj][0];
                let dy = vecs[ii][1] - vecs[jj][1];
                let dz = vecs[ii][2] - vecs[jj][2];
                let d = (dx * dx + dy * dy + dz * dz).sqrt();
                if d > f64::EPSILON {
                    log_d_sum[step] += d.ln();
                    log_d_cnt[step] += 1;
                }
            }
        }
    }
    let mut xs: Vec<f64> = Vec::new();
    let mut ys: Vec<f64> = Vec::new();
    for step in 0..max_steps {
        if log_d_cnt[step] > 5 {
            xs.push(step as f64);
            ys.push(log_d_sum[step] / log_d_cnt[step] as f64);
        }
    }
    if xs.len() < 5 {
        return LyapunovSnapshot {
            symbol: sym,
            as_of: as_of.into(),
            lyapunov_label: "INSUFFICIENT_DATA".into(),
            note: "too few regression points".into(),
            ..Default::default()
        };
    }
    let mlen = xs.len() as f64;
    let mx: f64 = xs.iter().sum::<f64>() / mlen;
    let my: f64 = ys.iter().sum::<f64>() / mlen;
    let mut sxx = 0.0f64;
    let mut sxy = 0.0f64;
    let mut syy = 0.0f64;
    for i in 0..xs.len() {
        let dx = xs[i] - mx;
        let dy = ys[i] - my;
        sxx += dx * dx;
        sxy += dx * dy;
        syy += dy * dy;
    }
    if sxx < f64::EPSILON {
        return LyapunovSnapshot {
            symbol: sym,
            as_of: as_of.into(),
            lyapunov_label: "INSUFFICIENT_DATA".into(),
            note: "degenerate regression".into(),
            ..Default::default()
        };
    }
    let lambda = sxy / sxx;
    let r2 = if syy > f64::EPSILON {
        (sxy * sxy) / (sxx * syy)
    } else {
        0.0
    };
    let label = if lambda > 0.10 {
        "CHAOTIC"
    } else if lambda > 0.02 {
        "WEAKLY_CHAOTIC"
    } else if lambda > -0.02 {
        "PERIODIC"
    } else {
        "STABLE"
    };
    LyapunovSnapshot {
        symbol: sym,
        as_of: as_of.into(),
        bars_used: n,
        embed_dim: m,
        time_delay: tau,
        lambda_max: lambda,
        r_squared: r2,
        steps_used: xs.len(),
        lyapunov_label: label.into(),
        note: String::new(),
    }
}

/// RANKAC compute: Spearman rank autocorrelation at lags 1, 5, 10.
pub fn compute_rankac_snapshot(
    symbol: &str,
    as_of: &str,
    bars: &[HistoricalPriceRow],
) -> RankacSnapshot {
    let sym = symbol.to_uppercase();
    let (_, log_rets) = trailing_log_returns(bars);
    if log_rets.len() < 30 {
        return RankacSnapshot {
            symbol: sym,
            as_of: as_of.into(),
            rankac_label: "INSUFFICIENT_DATA".into(),
            note: format!("need ≥30 returns, got {}", log_rets.len()),
            ..Default::default()
        };
    }
    let n = log_rets.len();
    // Compute ranks (average rank for ties, Spearman-style)
    let mut indexed: Vec<(usize, f64)> =
        log_rets.iter().enumerate().map(|(i, &v)| (i, v)).collect();
    indexed.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal));
    let mut ranks = vec![0.0f64; n];
    let mut i = 0;
    while i < n {
        let mut j = i;
        while j + 1 < n && (indexed[j + 1].1 - indexed[i].1).abs() < f64::EPSILON {
            j += 1;
        }
        let avg_rank = ((i + j) as f64) / 2.0 + 1.0;
        for k in i..=j {
            ranks[indexed[k].0] = avg_rank;
        }
        i = j + 1;
    }
    let compute_rho = |lag: usize| -> f64 {
        if lag >= n {
            return 0.0;
        }
        let m = n - lag;
        let mf = m as f64;
        let mut mx = 0.0f64;
        let mut my = 0.0f64;
        for i in 0..m {
            mx += ranks[i];
            my += ranks[i + lag];
        }
        mx /= mf;
        my /= mf;
        let mut sxx = 0.0f64;
        let mut syy = 0.0f64;
        let mut sxy = 0.0f64;
        for i in 0..m {
            let dx = ranks[i] - mx;
            let dy = ranks[i + lag] - my;
            sxx += dx * dx;
            syy += dy * dy;
            sxy += dx * dy;
        }
        if sxx < f64::EPSILON || syy < f64::EPSILON {
            0.0
        } else {
            sxy / (sxx.sqrt() * syy.sqrt())
        }
    };
    let r1 = compute_rho(1);
    let r5 = compute_rho(5);
    let r10 = compute_rho(10);
    let mean_abs = (r1.abs() + r5.abs() + r10.abs()) / 3.0;
    let max_abs = r1.abs().max(r5.abs()).max(r10.abs());
    let label = if max_abs > 0.30 {
        "STRONG_DEPENDENCE"
    } else if max_abs > 0.15 {
        "MODERATE_DEPENDENCE"
    } else if max_abs > 0.05 {
        "WEAK_DEPENDENCE"
    } else {
        "INDEPENDENT"
    };
    RankacSnapshot {
        symbol: sym,
        as_of: as_of.into(),
        bars_used: n,
        rho_lag1: r1,
        rho_lag5: r5,
        rho_lag10: r10,
        mean_abs_rho: mean_abs,
        max_abs_rho: max_abs,
        rankac_label: label.into(),
        note: String::new(),
    }
}

// Jump-test, Phillips-Perron, MF-DFA, Hill-KS, and trend-strength computes

/// BNSJUMP compute: Barndorff-Nielsen & Shephard 2006 jump-test Z-statistic.
pub fn compute_bnsjump_snapshot(
    symbol: &str,
    as_of: &str,
    bars: &[HistoricalPriceRow],
) -> BnsjumpSnapshot {
    let sym = symbol.to_uppercase();
    let (_, log_rets) = trailing_log_returns(bars);
    if log_rets.len() < 30 {
        return BnsjumpSnapshot {
            symbol: sym,
            as_of: as_of.into(),
            bnsjump_label: "INSUFFICIENT_DATA".into(),
            note: format!("need ≥30 returns, got {}", log_rets.len()),
            ..Default::default()
        };
    }
    let n = log_rets.len();
    // Realized variance RV = Σ r_i²
    let rv: f64 = log_rets.iter().map(|r| r * r).sum();
    // Bipower variation BV = (π/2) · Σ |r_i|·|r_{i-1}|
    let mu1_sq_inv = std::f64::consts::FRAC_PI_2; // 1/μ₁² for normal μ₁=√(2/π)
    let mut bv_sum = 0.0f64;
    for i in 1..n {
        bv_sum += log_rets[i - 1].abs() * log_rets[i].abs();
    }
    let bv = mu1_sq_inv * bv_sum;
    if rv < f64::EPSILON {
        return BnsjumpSnapshot {
            symbol: sym,
            as_of: as_of.into(),
            bnsjump_label: "INSUFFICIENT_DATA".into(),
            note: "zero realised variance".into(),
            ..Default::default()
        };
    }
    let jump_ratio = ((rv - bv) / rv).max(0.0);
    // Quarticity proxy for standardisation: (π²/4 + π − 5) · Σ r_i⁴
    let theta = (std::f64::consts::PI * std::f64::consts::PI) / 4.0 + std::f64::consts::PI - 5.0;
    let qv: f64 = log_rets.iter().map(|r| r.powi(4)).sum();
    let var_term = theta * qv;
    let z_stat = if var_term > f64::EPSILON {
        (rv - bv) / var_term.sqrt()
    } else {
        0.0
    };
    // Approx p-value using a rough normal CDF (Abramowitz-Stegun 26.2.17)
    fn norm_cdf(x: f64) -> f64 {
        let t = 1.0 / (1.0 + 0.2316419 * x.abs());
        let d = (-x * x / 2.0).exp() / (2.0 * std::f64::consts::PI).sqrt();
        let poly = (((1.330274429 * t - 1.821255978) * t + 1.781477937) * t - 0.356563782) * t
            + 0.319381530;
        let rhs = d * poly * t;
        if x >= 0.0 { 1.0 - rhs } else { rhs }
    }
    let p_value = (1.0 - norm_cdf(z_stat.abs())).max(0.0).min(1.0);
    let label = if z_stat > 3.09 {
        "STRONG_JUMP"
    } else if z_stat > 2.33 {
        "MODERATE_JUMP"
    } else if z_stat > 1.65 {
        "WEAK_JUMP"
    } else {
        "NO_JUMP"
    };
    BnsjumpSnapshot {
        symbol: sym,
        as_of: as_of.into(),
        bars_used: n,
        realized_variance: rv,
        bipower_variance: bv,
        jump_ratio,
        jump_z_stat: z_stat,
        p_value,
        bnsjump_label: label.into(),
        note: String::new(),
    }
}

/// PPROOT compute: Phillips-Perron 1988 nonparametric unit-root test.
pub fn compute_pproot_snapshot(
    symbol: &str,
    as_of: &str,
    bars: &[HistoricalPriceRow],
) -> PprootSnapshot {
    let sym = symbol.to_uppercase();
    if bars.len() < 30 {
        return PprootSnapshot {
            symbol: sym,
            as_of: as_of.into(),
            pproot_label: "INSUFFICIENT_DATA".into(),
            note: format!("need ≥30 bars, got {}", bars.len()),
            ..Default::default()
        };
    }
    // Use log-price series (the level process that might contain a unit root).
    let prices: Vec<f64> = bars
        .iter()
        .filter_map(|b| {
            if b.close > 0.0 {
                Some(b.close.ln())
            } else {
                None
            }
        })
        .collect();
    if prices.len() < 30 {
        return PprootSnapshot {
            symbol: sym,
            as_of: as_of.into(),
            pproot_label: "INSUFFICIENT_DATA".into(),
            note: format!("need ≥30 positive closes, got {}", prices.len()),
            ..Default::default()
        };
    }
    let n = prices.len();
    // OLS: Δy_t = (ρ − 1)·y_{t-1} + u_t  — estimate ρ directly from y_t on y_{t-1}
    let mut sum_xy = 0.0f64;
    let mut sum_xx = 0.0f64;
    for t in 1..n {
        let yl = prices[t - 1];
        let yc = prices[t];
        sum_xy += yl * yc;
        sum_xx += yl * yl;
    }
    if sum_xx < f64::EPSILON {
        return PprootSnapshot {
            symbol: sym,
            as_of: as_of.into(),
            pproot_label: "INSUFFICIENT_DATA".into(),
            note: "degenerate regressor".into(),
            ..Default::default()
        };
    }
    let rho_hat = sum_xy / sum_xx;
    let m = (n - 1) as f64;
    // Residuals û_t
    let mut resid: Vec<f64> = Vec::with_capacity(n - 1);
    let mut rss = 0.0f64;
    for t in 1..n {
        let u = prices[t] - rho_hat * prices[t - 1];
        rss += u * u;
        resid.push(u);
    }
    let sigma2 = rss / m;
    let se_rho = (sigma2 / sum_xx).sqrt().max(f64::EPSILON);
    let t_rho = (rho_hat - 1.0) / se_rho;
    // Long-run variance via Newey-West / Bartlett kernel, lag truncation q = floor(4·(n/100)^0.25)
    let q = ((4.0 * (m / 100.0).powf(0.25)).floor() as usize).max(1);
    let gamma0 = sigma2;
    let mut sigma2_lr = gamma0;
    for j in 1..=q {
        if j >= resid.len() {
            break;
        }
        let mut gamma_j = 0.0f64;
        for t in j..resid.len() {
            gamma_j += resid[t] * resid[t - j];
        }
        gamma_j /= m;
        let w = 1.0 - (j as f64) / ((q + 1) as f64);
        sigma2_lr += 2.0 * w * gamma_j;
    }
    let sigma2_lr = sigma2_lr.max(f64::EPSILON);
    // PP Z(ρ) and Z(t) corrections
    let z_rho = m * (rho_hat - 1.0) - 0.5 * m * m * (sigma2_lr - gamma0) / sum_xx;
    let z_t = (gamma0 / sigma2_lr).sqrt() * t_rho
        - 0.5 * (sigma2_lr - gamma0) * (m * se_rho / sigma2_lr.sqrt()) / sigma2_lr.sqrt();
    // Dickey-Fuller critical values for Z(t), no-trend case
    let label = if z_t < -3.43 {
        "STATIONARY_STRONG"
    } else if z_t < -2.86 {
        "STATIONARY_WEAK"
    } else if z_t < -2.57 {
        "BORDERLINE"
    } else {
        "UNIT_ROOT"
    };
    PprootSnapshot {
        symbol: sym,
        as_of: as_of.into(),
        bars_used: n,
        rho_hat,
        t_rho,
        z_rho,
        z_t,
        lag_truncation: q,
        pproot_label: label.into(),
        note: String::new(),
    }
}

/// MFDFA compute: Multifractal DFA at q ∈ {−2, 0, +2}.
pub fn compute_mfdfa_snapshot(
    symbol: &str,
    as_of: &str,
    bars: &[HistoricalPriceRow],
) -> MfdfaSnapshot {
    let sym = symbol.to_uppercase();
    let (_, log_rets) = trailing_log_returns(bars);
    if log_rets.len() < 120 {
        return MfdfaSnapshot {
            symbol: sym,
            as_of: as_of.into(),
            mfdfa_label: "INSUFFICIENT_DATA".into(),
            note: format!("need ≥120 returns, got {}", log_rets.len()),
            ..Default::default()
        };
    }
    let n = log_rets.len();
    // Cumulative-sum walk Y_k = Σ_{i≤k} (r_i − r̄)
    let rbar: f64 = log_rets.iter().sum::<f64>() / (n as f64);
    let mut y = Vec::with_capacity(n);
    let mut cum = 0.0f64;
    for &r in &log_rets {
        cum += r - rbar;
        y.push(cum);
    }
    // Scales: s = 8, 12, 16, 24, 32, 48, 64 (bounded by n/4)
    let scales: Vec<usize> = [8usize, 12, 16, 24, 32, 48, 64]
        .iter()
        .copied()
        .filter(|&s| s * 4 <= n)
        .collect();
    if scales.len() < 3 {
        return MfdfaSnapshot {
            symbol: sym,
            as_of: as_of.into(),
            mfdfa_label: "INSUFFICIENT_DATA".into(),
            note: "too few viable scales".into(),
            ..Default::default()
        };
    }
    // For each scale, split walk into non-overlapping windows, fit linear detrend, compute F²(s,v).
    // Then aggregate: F_q(s) = { (1/N_s) Σ [F²(s,v)]^(q/2) }^(1/q)  (q ≠ 0)
    //                 F_0(s) = exp{ (1/(2 N_s)) Σ ln F²(s,v) }     (q = 0)
    // Fit h(q) as slope of ln F_q(s) vs ln s.
    let compute_hq = |q: f64| -> Option<f64> {
        let mut log_s = Vec::new();
        let mut log_f = Vec::new();
        for &s in &scales {
            let ns = n / s;
            if ns < 4 {
                continue;
            }
            let mut f2_vals = Vec::with_capacity(ns * 2);
            for direction in 0..2usize {
                for v in 0..ns {
                    let offset = if direction == 0 {
                        v * s
                    } else {
                        n - (v + 1) * s
                    };
                    // Linear detrend over y[offset..offset+s]
                    let sf = s as f64;
                    let mut sx = 0.0f64;
                    let mut sy = 0.0f64;
                    for k in 0..s {
                        sx += k as f64;
                        sy += y[offset + k];
                    }
                    let mx = sx / sf;
                    let my = sy / sf;
                    let mut sxx = 0.0f64;
                    let mut sxy = 0.0f64;
                    for k in 0..s {
                        let dx = (k as f64) - mx;
                        let dy = y[offset + k] - my;
                        sxx += dx * dx;
                        sxy += dx * dy;
                    }
                    let slope = if sxx > f64::EPSILON { sxy / sxx } else { 0.0 };
                    let intercept = my - slope * mx;
                    let mut ss = 0.0f64;
                    for k in 0..s {
                        let fitted = intercept + slope * (k as f64);
                        let d = y[offset + k] - fitted;
                        ss += d * d;
                    }
                    f2_vals.push((ss / sf).max(f64::EPSILON));
                }
            }
            if f2_vals.is_empty() {
                continue;
            }
            let nv = f2_vals.len() as f64;
            let fq = if q.abs() < f64::EPSILON {
                (f2_vals.iter().map(|v| v.ln()).sum::<f64>() / (2.0 * nv)).exp()
            } else {
                let m: f64 = f2_vals.iter().map(|v| v.powf(q / 2.0)).sum::<f64>() / nv;
                m.powf(1.0 / q)
            };
            if fq.is_finite() && fq > 0.0 {
                log_s.push((s as f64).ln());
                log_f.push(fq.ln());
            }
        }
        if log_s.len() < 3 {
            return None;
        }
        let ln = log_s.len() as f64;
        let mx = log_s.iter().sum::<f64>() / ln;
        let my = log_f.iter().sum::<f64>() / ln;
        let mut sxx = 0.0f64;
        let mut sxy = 0.0f64;
        for i in 0..log_s.len() {
            let dx = log_s[i] - mx;
            let dy = log_f[i] - my;
            sxx += dx * dx;
            sxy += dx * dy;
        }
        if sxx < f64::EPSILON {
            None
        } else {
            Some(sxy / sxx)
        }
    };
    let h_neg2 = compute_hq(-2.0);
    let h_zero = compute_hq(0.0);
    let h_pos2 = compute_hq(2.0);
    let (h_n, h_0, h_p) = match (h_neg2, h_zero, h_pos2) {
        (Some(a), Some(b), Some(c)) => (a, b, c),
        _ => {
            return MfdfaSnapshot {
                symbol: sym,
                as_of: as_of.into(),
                mfdfa_label: "INSUFFICIENT_DATA".into(),
                note: "h(q) regression failed".into(),
                ..Default::default()
            };
        }
    };
    let delta_h = h_n - h_p;
    let label = if delta_h > 0.30 {
        "STRONG_MULTIFRACTAL"
    } else if delta_h > 0.15 {
        "MODERATE_MULTIFRACTAL"
    } else if delta_h > 0.05 {
        "WEAK_MULTIFRACTAL"
    } else {
        "MONOFRACTAL"
    };
    MfdfaSnapshot {
        symbol: sym,
        as_of: as_of.into(),
        bars_used: n,
        h_q_neg2: h_n,
        h_q_zero: h_0,
        h_q_pos2: h_p,
        delta_h,
        scales_used: scales.len(),
        mfdfa_label: label.into(),
        note: String::new(),
    }
}

/// HILLKS compute: KS goodness-of-fit for Hill-tail Pareto.
pub fn compute_hillks_snapshot(
    symbol: &str,
    as_of: &str,
    bars: &[HistoricalPriceRow],
) -> HillksSnapshot {
    let sym = symbol.to_uppercase();
    let (_, log_rets) = trailing_log_returns(bars);
    if log_rets.len() < 50 {
        return HillksSnapshot {
            symbol: sym,
            as_of: as_of.into(),
            hillks_label: "INSUFFICIENT_DATA".into(),
            note: format!("need ≥50 returns, got {}", log_rets.len()),
            ..Default::default()
        };
    }
    let n = log_rets.len();
    // Use absolute log-returns as tail sample (two-sided symmetric tail model).
    let mut abs_r: Vec<f64> = log_rets
        .iter()
        .map(|r| r.abs())
        .filter(|v| *v > f64::EPSILON)
        .collect();
    abs_r.sort_by(|a, b| b.partial_cmp(a).unwrap_or(std::cmp::Ordering::Equal)); // descending
    let k = (n as f64 * 0.10).floor() as usize;
    if k < 10 || k >= abs_r.len() {
        return HillksSnapshot {
            symbol: sym,
            as_of: as_of.into(),
            hillks_label: "INSUFFICIENT_DATA".into(),
            note: "tail sample too small".into(),
            ..Default::default()
        };
    }
    // Hill estimator of α: 1/α̂ = (1/k) Σ_{i=1..k} ln(x_i / x_{k+1})
    let threshold = abs_r[k];
    if threshold < f64::EPSILON {
        return HillksSnapshot {
            symbol: sym,
            as_of: as_of.into(),
            hillks_label: "INSUFFICIENT_DATA".into(),
            note: "zero threshold".into(),
            ..Default::default()
        };
    }
    let mut inv_alpha = 0.0f64;
    for i in 0..k {
        inv_alpha += (abs_r[i] / threshold).ln();
    }
    inv_alpha /= k as f64;
    if inv_alpha < f64::EPSILON {
        return HillksSnapshot {
            symbol: sym,
            as_of: as_of.into(),
            hillks_label: "INSUFFICIENT_DATA".into(),
            note: "degenerate tail".into(),
            ..Default::default()
        };
    }
    let alpha = 1.0 / inv_alpha;
    // KS statistic between empirical CDF of (x_i / threshold) for i=1..k and Pareto(α) CDF F(y) = 1 − y^{−α}.
    // Sort tail sample x_1..x_k in ascending order.
    let mut tail: Vec<f64> = abs_r[..k].to_vec();
    tail.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    let mut d_max = 0.0f64;
    for (i, &x) in tail.iter().enumerate() {
        let y = x / threshold;
        if y < 1.0 - f64::EPSILON {
            continue;
        }
        let f_model = 1.0 - y.powf(-alpha);
        let f_emp_lo = i as f64 / k as f64;
        let f_emp_hi = (i + 1) as f64 / k as f64;
        d_max = d_max
            .max((f_emp_lo - f_model).abs())
            .max((f_emp_hi - f_model).abs());
    }
    let ks_crit = 1.36 / (k as f64).sqrt();
    let label = if d_max < ks_crit * 0.50 {
        "GOOD_FIT"
    } else if d_max < ks_crit * 0.90 {
        "ACCEPTABLE_FIT"
    } else if d_max < ks_crit * 1.30 {
        "POOR_FIT"
    } else {
        "REJECT"
    };
    HillksSnapshot {
        symbol: sym,
        as_of: as_of.into(),
        bars_used: n,
        k_order: k,
        alpha_hat: alpha,
        ks_statistic: d_max,
        ks_critical_5pct: ks_crit,
        hillks_label: label.into(),
        note: String::new(),
    }
}

/// TSI compute: Blau 1991 True Strength Index.
pub fn compute_tsi_snapshot(symbol: &str, as_of: &str, bars: &[HistoricalPriceRow]) -> TsiSnapshot {
    let sym = symbol.to_uppercase();
    let closes: Vec<f64> = bars
        .iter()
        .filter_map(|b| if b.close > 0.0 { Some(b.close) } else { None })
        .collect();
    if closes.len() < 60 {
        return TsiSnapshot {
            symbol: sym,
            as_of: as_of.into(),
            tsi_label: "INSUFFICIENT_DATA".into(),
            note: format!("need ≥60 closes, got {}", closes.len()),
            ..Default::default()
        };
    }
    let n = closes.len();
    let long_p = 25usize;
    let short_p = 13usize;
    let diffs: Vec<f64> = (1..n).map(|i| closes[i] - closes[i - 1]).collect();
    let abs_diffs: Vec<f64> = diffs.iter().map(|d| d.abs()).collect();
    // EMA helper: EMA(x, p) where α = 2/(p+1); seed with first value.
    fn ema_series(x: &[f64], p: usize) -> Vec<f64> {
        if x.is_empty() {
            return Vec::new();
        }
        let alpha = 2.0 / ((p + 1) as f64);
        let mut out = Vec::with_capacity(x.len());
        out.push(x[0]);
        for i in 1..x.len() {
            out.push(alpha * x[i] + (1.0 - alpha) * out[i - 1]);
        }
        out
    }
    let long_smooth_num = ema_series(&diffs, long_p);
    let double_num = ema_series(&long_smooth_num, short_p);
    let long_smooth_den = ema_series(&abs_diffs, long_p);
    let double_den = ema_series(&long_smooth_den, short_p);
    let last = diffs.len() - 1;
    let den = double_den[last];
    if den.abs() < f64::EPSILON {
        return TsiSnapshot {
            symbol: sym,
            as_of: as_of.into(),
            tsi_label: "INSUFFICIENT_DATA".into(),
            note: "flat tape".into(),
            ..Default::default()
        };
    }
    let tsi_series: Vec<f64> = double_num
        .iter()
        .zip(double_den.iter())
        .map(|(n, d)| {
            if d.abs() < f64::EPSILON {
                0.0
            } else {
                100.0 * n / d
            }
        })
        .collect();
    let signal_series = ema_series(&tsi_series, short_p);
    let tsi = tsi_series[last];
    let signal = signal_series[last];
    let diff = tsi - signal;
    let label = if tsi > 25.0 {
        "STRONG_BULL"
    } else if tsi > 0.0 {
        "BULL"
    } else if tsi > -25.0 {
        if tsi > -5.0 && tsi < 5.0 {
            "NEUTRAL"
        } else {
            "BEAR"
        }
    } else {
        "STRONG_BEAR"
    };
    TsiSnapshot {
        symbol: sym,
        as_of: as_of.into(),
        bars_used: n,
        ema_long: long_p,
        ema_short: short_p,
        tsi_value: tsi,
        signal_value: signal,
        tsi_minus_signal: diff,
        tsi_label: label.into(),
        note: String::new(),
    }
}

// GARCH, SADF, correlation-dimension, spectral-skew, and automutual-information computes

/// GARCH11 compute: Bollerslev 1986 GARCH(1,1) fit via coordinate-descent MLE.
pub fn compute_garch11_snapshot(
    symbol: &str,
    as_of: &str,
    bars: &[HistoricalPriceRow],
) -> Garch11Snapshot {
    let sym = symbol.to_uppercase();
    let (_, log_rets) = trailing_log_returns(bars);
    if log_rets.len() < 60 {
        return Garch11Snapshot {
            symbol: sym,
            as_of: as_of.into(),
            garch11_label: "INSUFFICIENT_DATA".into(),
            note: format!("need ≥60 returns, got {}", log_rets.len()),
            ..Default::default()
        };
    }
    let n = log_rets.len();
    let mean_r: f64 = log_rets.iter().sum::<f64>() / n as f64;
    let resid: Vec<f64> = log_rets.iter().map(|r| r - mean_r).collect();
    let sample_var: f64 = resid.iter().map(|r| r * r).sum::<f64>() / n as f64;
    if sample_var < f64::EPSILON {
        return Garch11Snapshot {
            symbol: sym,
            as_of: as_of.into(),
            garch11_label: "INSUFFICIENT_DATA".into(),
            note: "zero sample variance".into(),
            ..Default::default()
        };
    }
    // Evaluate the GARCH(1,1) Gaussian log-likelihood for a candidate (ω, α, β).
    let log_lik = |omega: f64, alpha: f64, beta: f64| -> f64 {
        if omega <= 0.0 || alpha < 0.0 || beta < 0.0 || alpha + beta >= 0.999 {
            return f64::NEG_INFINITY;
        }
        let mut sigma2 = sample_var;
        let mut ll = 0.0f64;
        for r in &resid {
            if sigma2 <= 0.0 {
                return f64::NEG_INFINITY;
            }
            ll += -0.5 * ((2.0 * std::f64::consts::PI * sigma2).ln() + (r * r) / sigma2);
            sigma2 = omega + alpha * r * r + beta * sigma2;
        }
        ll
    };
    // Coarse grid search over (α, β) with ω implied by the unconditional-variance constraint.
    let mut best = (0.05f64, 0.90f64, f64::NEG_INFINITY);
    let alphas: Vec<f64> = (1..=20).map(|i| i as f64 * 0.02).collect(); // 0.02..0.40
    let betas: Vec<f64> = (1..=95).map(|i| i as f64 * 0.01).collect(); // 0.01..0.95
    for &a in &alphas {
        for &b in &betas {
            if a + b >= 0.995 {
                continue;
            }
            let omega = sample_var * (1.0 - a - b);
            let ll = log_lik(omega, a, b);
            if ll.is_finite() && ll > best.2 {
                best = (a, b, ll);
            }
        }
    }
    if !best.2.is_finite() {
        return Garch11Snapshot {
            symbol: sym,
            as_of: as_of.into(),
            garch11_label: "INSUFFICIENT_DATA".into(),
            note: "grid search failed".into(),
            ..Default::default()
        };
    }
    let (alpha, beta, ll) = best;
    let omega = sample_var * (1.0 - alpha - beta);
    let persistence = alpha + beta;
    let unc_var = if persistence < 1.0 {
        omega / (1.0 - persistence)
    } else {
        f64::NAN
    };
    let half_life = if persistence < 1.0 && persistence > 0.0 {
        (0.5_f64.ln()) / persistence.ln()
    } else {
        f64::NAN
    };
    let label = if persistence > 0.98 {
        "NEAR_INTEGRATED"
    } else if persistence > 0.90 {
        "HIGH_PERSISTENCE"
    } else if persistence > 0.70 {
        "MODERATE_PERSISTENCE"
    } else {
        "LOW_PERSISTENCE"
    };
    Garch11Snapshot {
        symbol: sym,
        as_of: as_of.into(),
        bars_used: n,
        omega,
        alpha,
        beta,
        persistence,
        unconditional_var: unc_var,
        half_life_bars: half_life,
        log_likelihood: ll,
        garch11_label: label.into(),
        note: String::new(),
    }
}

/// SADF compute: Phillips-Wu-Yu 2011 Sup-ADF explosive-root test on log-prices.
pub fn compute_sadf_snapshot(
    symbol: &str,
    as_of: &str,
    bars: &[HistoricalPriceRow],
) -> SadfSnapshot {
    let sym = symbol.to_uppercase();
    if bars.len() < 60 {
        return SadfSnapshot {
            symbol: sym,
            as_of: as_of.into(),
            sadf_label: "INSUFFICIENT_DATA".into(),
            note: format!("need ≥60 bars, got {}", bars.len()),
            ..Default::default()
        };
    }
    let prices: Vec<f64> = bars
        .iter()
        .filter_map(|b| {
            if b.close > 0.0 {
                Some(b.close.ln())
            } else {
                None
            }
        })
        .collect();
    let n = prices.len();
    if n < 60 {
        return SadfSnapshot {
            symbol: sym,
            as_of: as_of.into(),
            sadf_label: "INSUFFICIENT_DATA".into(),
            note: format!("need ≥60 positive closes, got {}", n),
            ..Default::default()
        };
    }
    // ADF-t on series prices[0..end]: regression Δy_t = β·y_{t-1} + c + ε, return t-stat for β.
    let adf_t = |series: &[f64]| -> Option<f64> {
        let m = series.len();
        if m < 10 {
            return None;
        }
        // Build Δy and regressors (y_{t-1}, constant)
        let mut sx1 = 0.0f64; // Σy_{t-1}
        let mut sx1x1 = 0.0f64;
        let mut sx1x2 = 0.0f64;
        let mut sx2x2 = 0.0f64;
        let mut sx1dy = 0.0f64;
        let mut sx2dy = 0.0f64;
        let mm = (m - 1) as f64;
        for i in 1..m {
            let x1 = series[i - 1];
            let x2 = 1.0;
            let dy = series[i] - series[i - 1];
            sx1 += x1;
            sx1x1 += x1 * x1;
            sx1x2 += x1 * x2;
            sx2x2 += x2 * x2;
            sx1dy += x1 * dy;
            sx2dy += x2 * dy;
        }
        let _ = sx1;
        // Normal equations for 2-var OLS (x1, x2)
        let det = sx1x1 * sx2x2 - sx1x2 * sx1x2;
        if det.abs() < 1e-12 {
            return None;
        }
        let beta = (sx2x2 * sx1dy - sx1x2 * sx2dy) / det;
        let cons = (-sx1x2 * sx1dy + sx1x1 * sx2dy) / det;
        // Residuals → σ² → SE(β)
        let mut rss = 0.0f64;
        for i in 1..m {
            let x1 = series[i - 1];
            let dy = series[i] - series[i - 1];
            let e = dy - beta * x1 - cons;
            rss += e * e;
        }
        let dfree = mm - 2.0;
        if dfree <= 0.0 {
            return None;
        }
        let sigma2 = rss / dfree;
        let var_beta = sigma2 * sx2x2 / det;
        if var_beta <= 0.0 {
            return None;
        }
        Some(beta / var_beta.sqrt())
    };
    let adf_full = adf_t(&prices).unwrap_or(0.0);
    // Sup-ADF: expand from r0 to n
    let r0 = ((0.01 + 1.8 / (n as f64).sqrt()) * n as f64).floor() as usize;
    let r0 = r0.max(20).min(n - 1);
    let mut sadf = f64::NEG_INFINITY;
    let mut sadf_end = r0;
    let mut end = r0;
    while end <= n {
        if let Some(t) = adf_t(&prices[..end]) {
            if t > sadf {
                sadf = t;
                sadf_end = end;
            }
        }
        end += 1;
    }
    if !sadf.is_finite() {
        return SadfSnapshot {
            symbol: sym,
            as_of: as_of.into(),
            sadf_label: "INSUFFICIENT_DATA".into(),
            note: "SADF regression failed".into(),
            ..Default::default()
        };
    }
    // Conservative 95% critical via small lookup (interpolated). Values for the standard no-trend SADF from PWY 2011.
    let crit = match n {
        0..=100 => 1.35,
        101..=200 => 1.49,
        201..=400 => 1.57,
        _ => 1.63,
    };
    let reject = sadf > crit;
    let label = if sadf > crit + 0.5 {
        "EXPLOSIVE_CONFIRMED"
    } else if sadf > crit {
        "EXPLOSIVE_LIKELY"
    } else if sadf > crit - 0.3 {
        "BORDERLINE"
    } else {
        "STABLE"
    };
    SadfSnapshot {
        symbol: sym,
        as_of: as_of.into(),
        bars_used: n,
        min_window: r0,
        adf_full,
        sadf_stat: sadf,
        sadf_argmax_end: sadf_end,
        critical_95: crit,
        reject_null: reject,
        sadf_label: label.into(),
        note: String::new(),
    }
}

/// CORDIM compute: Grassberger-Procaccia 1983 correlation dimension D2.
pub fn compute_cordim_snapshot(
    symbol: &str,
    as_of: &str,
    bars: &[HistoricalPriceRow],
) -> CordimSnapshot {
    let sym = symbol.to_uppercase();
    let (_, log_rets) = trailing_log_returns(bars);
    if log_rets.len() < 60 {
        return CordimSnapshot {
            symbol: sym,
            as_of: as_of.into(),
            cordim_label: "INSUFFICIENT_DATA".into(),
            note: format!("need ≥60 returns, got {}", log_rets.len()),
            ..Default::default()
        };
    }
    let n = log_rets.len();
    let m = 3usize;
    let tau = 1usize;
    if n <= (m - 1) * tau + 2 {
        return CordimSnapshot {
            symbol: sym,
            as_of: as_of.into(),
            cordim_label: "INSUFFICIENT_DATA".into(),
            note: "too few embedded vectors".into(),
            ..Default::default()
        };
    }
    // Standardise so that radii are on a consistent scale
    let mean_r: f64 = log_rets.iter().sum::<f64>() / n as f64;
    let var_r: f64 = log_rets.iter().map(|r| (r - mean_r).powi(2)).sum::<f64>() / n as f64;
    let std_r = var_r.sqrt().max(f64::EPSILON);
    let z: Vec<f64> = log_rets.iter().map(|r| (r - mean_r) / std_r).collect();
    // Build embedded vectors
    let nv = n - (m - 1) * tau;
    let vecs: Vec<[f64; 3]> = (0..nv)
        .map(|i| [z[i], z[i + tau], z[i + 2 * tau]])
        .collect();
    // Choose log-spaced radii ε between 0.1 and 2.0 (standardised units), 10 points
    let log_radii: Vec<f64> = (0..10)
        .map(|i| -1.0 + (i as f64) * (0.3010 / 10.0 * 10.0))
        .collect(); // log10 spacing 0.0..0.3 would be too narrow
    // Simpler: ε in {0.10, 0.14, 0.20, 0.28, 0.40, 0.56, 0.79, 1.12, 1.58, 2.24} (geometric)
    let radii: Vec<f64> = (0..10)
        .map(|i| 0.10 * (10f64.powf(i as f64 / 10.0)))
        .collect(); // log10-spaced 0.1 → 10^(0.9) ≈ 0.79 ... actually goes to 1.0
    // Use that set.
    let _ = log_radii; // keep unused (prototype kept for docs)
    let nv_f = (nv * (nv - 1)) as f64; // denominator for C(ε): pairs (i<j)
    let mut log_eps: Vec<f64> = Vec::new();
    let mut log_c: Vec<f64> = Vec::new();
    for &eps in &radii {
        let eps2 = eps * eps;
        let mut count = 0usize;
        for i in 0..nv {
            for j in (i + 1)..nv {
                let a = &vecs[i];
                let b = &vecs[j];
                let dx = a[0] - b[0];
                let dy = a[1] - b[1];
                let dz = a[2] - b[2];
                if dx * dx + dy * dy + dz * dz <= eps2 {
                    count += 1;
                }
            }
        }
        if count == 0 {
            continue;
        }
        let c = 2.0 * count as f64 / nv_f;
        if c > 0.0 {
            log_eps.push(eps.ln());
            log_c.push(c.ln());
        }
    }
    if log_eps.len() < 3 {
        return CordimSnapshot {
            symbol: sym,
            as_of: as_of.into(),
            embed_dim: m,
            cordim_label: "INSUFFICIENT_DATA".into(),
            note: "too few valid radii for fit".into(),
            ..Default::default()
        };
    }
    let ln = log_eps.len() as f64;
    let mx = log_eps.iter().sum::<f64>() / ln;
    let my = log_c.iter().sum::<f64>() / ln;
    let mut sxx = 0.0f64;
    let mut sxy = 0.0f64;
    let mut syy = 0.0f64;
    for i in 0..log_eps.len() {
        let dx = log_eps[i] - mx;
        let dy = log_c[i] - my;
        sxx += dx * dx;
        sxy += dx * dy;
        syy += dy * dy;
    }
    if sxx < f64::EPSILON {
        return CordimSnapshot {
            symbol: sym,
            as_of: as_of.into(),
            embed_dim: m,
            cordim_label: "INSUFFICIENT_DATA".into(),
            note: "zero ε spread".into(),
            ..Default::default()
        };
    }
    let d2 = sxy / sxx;
    let r2 = if syy > f64::EPSILON {
        (sxy * sxy) / (sxx * syy)
    } else {
        0.0
    };
    let label = if d2 < 1.5 {
        "LOW_DIM"
    } else if d2 < 2.5 {
        "MODERATE_DIM"
    } else if d2 < 3.0 {
        "HIGH_DIM"
    } else {
        "STOCHASTIC"
    };
    CordimSnapshot {
        symbol: sym,
        as_of: as_of.into(),
        bars_used: n,
        embed_dim: m,
        radii_count: log_eps.len(),
        d2,
        r_squared: r2,
        cordim_label: label.into(),
        note: String::new(),
    }
}

/// SKSPEC compute: Rolling-window skewness spectrum / stability.
pub fn compute_skspec_snapshot(
    symbol: &str,
    as_of: &str,
    bars: &[HistoricalPriceRow],
) -> SkspecSnapshot {
    let sym = symbol.to_uppercase();
    let (_, log_rets) = trailing_log_returns(bars);
    if log_rets.len() < 60 {
        return SkspecSnapshot {
            symbol: sym,
            as_of: as_of.into(),
            skspec_label: "INSUFFICIENT_DATA".into(),
            note: format!("need ≥60 returns, got {}", log_rets.len()),
            ..Default::default()
        };
    }
    let n = log_rets.len();
    let w = 30usize;
    if n < w * 2 {
        return SkspecSnapshot {
            symbol: sym,
            as_of: as_of.into(),
            window_size: w,
            skspec_label: "INSUFFICIENT_DATA".into(),
            note: "need ≥2 windows of returns".into(),
            ..Default::default()
        };
    }
    // Rolling skew on [i..i+w]
    let mut skews: Vec<f64> = Vec::with_capacity(n - w);
    for i in 0..=(n - w) {
        let slice = &log_rets[i..i + w];
        let wf = w as f64;
        let mean = slice.iter().sum::<f64>() / wf;
        let var = slice.iter().map(|r| (r - mean).powi(2)).sum::<f64>() / wf;
        if var < f64::EPSILON {
            continue;
        }
        let s = var.sqrt();
        let skew = slice.iter().map(|r| ((r - mean) / s).powi(3)).sum::<f64>() / wf;
        if skew.is_finite() {
            skews.push(skew);
        }
    }
    if skews.len() < 5 {
        return SkspecSnapshot {
            symbol: sym,
            as_of: as_of.into(),
            window_size: w,
            skspec_label: "INSUFFICIENT_DATA".into(),
            note: "too few valid rolling skew windows".into(),
            ..Default::default()
        };
    }
    let sk_n = skews.len() as f64;
    let mean_sk = skews.iter().sum::<f64>() / sk_n;
    let var_sk = skews.iter().map(|v| (v - mean_sk).powi(2)).sum::<f64>() / sk_n;
    let std_sk = var_sk.sqrt();
    let min_sk = skews.iter().cloned().fold(f64::INFINITY, f64::min);
    let max_sk = skews.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
    let range_sk = max_sk - min_sk;
    let label = if std_sk > 1.0 {
        "UNSTABLE"
    } else if mean_sk.abs() < 0.2 && std_sk > 0.5 {
        "DRIFTING"
    } else if mean_sk > 0.2 {
        "STABLE_POSITIVE"
    } else if mean_sk < -0.2 {
        "STABLE_NEGATIVE"
    } else {
        "DRIFTING"
    };
    SkspecSnapshot {
        symbol: sym,
        as_of: as_of.into(),
        bars_used: n,
        window_size: w,
        mean_skew: mean_sk,
        std_skew: std_sk,
        min_skew: min_sk,
        max_skew: max_sk,
        range_skew: range_sk,
        skspec_label: label.into(),
        note: String::new(),
    }
}

/// AUTOMI compute: Lag-1/5/10 auto-mutual-information (histogram-based).
pub fn compute_automi_snapshot(
    symbol: &str,
    as_of: &str,
    bars: &[HistoricalPriceRow],
) -> AutomiSnapshot {
    let sym = symbol.to_uppercase();
    let (_, log_rets) = trailing_log_returns(bars);
    if log_rets.len() < 50 {
        return AutomiSnapshot {
            symbol: sym,
            as_of: as_of.into(),
            automi_label: "INSUFFICIENT_DATA".into(),
            note: format!("need ≥50 returns, got {}", log_rets.len()),
            ..Default::default()
        };
    }
    let n = log_rets.len();
    let k = 8usize;
    // Equi-probable bin edges via sorted-order quantiles
    let mut sorted = log_rets.clone();
    sorted.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    let mut edges: Vec<f64> = Vec::with_capacity(k + 1);
    edges.push(f64::NEG_INFINITY);
    for b in 1..k {
        let idx = ((b as f64 / k as f64) * n as f64).floor() as usize;
        let idx = idx.min(n - 1);
        edges.push(sorted[idx]);
    }
    edges.push(f64::INFINITY);
    let bin_of = |x: f64| -> usize {
        for b in 0..k {
            if x >= edges[b] && x < edges[b + 1] {
                return b;
            }
        }
        k - 1
    };
    let bins: Vec<usize> = log_rets.iter().map(|&r| bin_of(r)).collect();
    // Marginal entropy H(X)
    let mut marg = vec![0usize; k];
    for &b in &bins {
        marg[b] += 1;
    }
    let nf = n as f64;
    let ln2 = std::f64::consts::LN_2;
    let h_x: f64 = marg
        .iter()
        .filter(|&&c| c > 0)
        .map(|&c| {
            let p = c as f64 / nf;
            -p * p.ln() / ln2
        })
        .sum();
    let compute_mi = |lag: usize| -> f64 {
        if lag >= n {
            return 0.0;
        }
        let m = n - lag;
        let mut joint = vec![vec![0usize; k]; k];
        let mut mx = vec![0usize; k];
        let mut my = vec![0usize; k];
        for i in 0..m {
            let a = bins[i];
            let b = bins[i + lag];
            joint[a][b] += 1;
            mx[a] += 1;
            my[b] += 1;
        }
        let mf = m as f64;
        let mut mi = 0.0f64;
        for a in 0..k {
            for b in 0..k {
                let pjoint = joint[a][b] as f64 / mf;
                if pjoint <= 0.0 {
                    continue;
                }
                let pa = mx[a] as f64 / mf;
                let pb = my[b] as f64 / mf;
                if pa <= 0.0 || pb <= 0.0 {
                    continue;
                }
                mi += pjoint * (pjoint / (pa * pb)).ln() / ln2;
            }
        }
        mi.max(0.0)
    };
    let mi1 = compute_mi(1);
    let mi5 = compute_mi(5);
    let mi10 = compute_mi(10);
    let norm_mi1 = if h_x > f64::EPSILON { mi1 / h_x } else { 0.0 };
    let label = if norm_mi1 > 0.25 {
        "STRONG"
    } else if norm_mi1 > 0.12 {
        "MODERATE"
    } else if norm_mi1 > 0.05 {
        "WEAK"
    } else {
        "INDEPENDENT"
    };
    AutomiSnapshot {
        symbol: sym,
        as_of: as_of.into(),
        bars_used: n,
        num_bins: k,
        mi_lag1: mi1,
        mi_lag5: mi5,
        mi_lag10: mi10,
        h_marginal: h_x,
        normalized_mi1: norm_mi1,
        automi_label: label.into(),
        note: String::new(),
    }
}
