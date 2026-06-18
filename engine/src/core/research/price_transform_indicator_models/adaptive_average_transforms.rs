use super::*;

// Adaptive averages, rainbow levels, cycle phase, and internal-bar-strength transforms

pub fn compute_wma_snapshot(symbol: &str, as_of: &str, bars: &[HistoricalPriceRow]) -> WmaSnapshot {
    let sym = symbol.to_uppercase();
    let mut sorted: Vec<&HistoricalPriceRow> = bars.iter().collect();
    sorted.sort_by(|a, b| a.date.cmp(&b.date));
    let n = sorted.len();
    let length = 20usize;
    if n < length + 1 {
        return WmaSnapshot {
            symbol: sym,
            as_of: as_of.into(),
            length,
            wma_label: "INSUFFICIENT_DATA".into(),
            note: format!("need ≥{} bars, got {}", length + 1, n),
            ..Default::default()
        };
    }
    let wma_at = |end_idx: usize| -> f64 {
        let mut num = 0.0_f64;
        let mut den = 0.0_f64;
        for k in 0..length {
            let w = (length - k) as f64;
            num += sorted[end_idx - k].close * w;
            den += w;
        }
        num / den
    };
    let wma = wma_at(n - 1);
    let wma_prev = wma_at(n - 2);
    let mut sma_sum = 0.0_f64;
    for k in 0..length {
        sma_sum += sorted[n - 1 - k].close;
    }
    let sma = sma_sum / length as f64;
    let close = sorted[n - 1].close;
    let spread = close - wma;
    let spread_pct = if wma.abs() > 1e-12 { spread / wma } else { 0.0 };
    let label = if spread_pct > 0.02 {
        "BULL"
    } else if spread_pct > 0.005 {
        "WEAK_BULL"
    } else if spread_pct < -0.02 {
        "BEAR"
    } else if spread_pct < -0.005 {
        "WEAK_BEAR"
    } else {
        "NEUTRAL"
    };
    WmaSnapshot {
        symbol: sym,
        as_of: as_of.into(),
        bars_used: n,
        length,
        wma_value: wma,
        wma_prev,
        sma_value: sma,
        spread,
        spread_pct,
        last_close: close,
        wma_label: label.into(),
        note: String::new(),
    }
}

pub fn compute_rainbow_snapshot(
    symbol: &str,
    as_of: &str,
    bars: &[HistoricalPriceRow],
) -> RainbowSnapshot {
    let sym = symbol.to_uppercase();
    let mut sorted: Vec<&HistoricalPriceRow> = bars.iter().collect();
    sorted.sort_by(|a, b| a.date.cmp(&b.date));
    let n = sorted.len();
    let levels = 10usize;
    let warmup = 2 * levels + 2;
    if n < warmup {
        return RainbowSnapshot {
            symbol: sym,
            as_of: as_of.into(),
            levels,
            rainbow_label: "INSUFFICIENT_DATA".into(),
            note: format!("need ≥{} bars, got {}", warmup, n),
            ..Default::default()
        };
    }
    let closes: Vec<f64> = sorted.iter().map(|b| b.close).collect();
    let mut series: Vec<Vec<f64>> = vec![closes.clone()];
    for _ in 0..levels {
        let prev = series.last().unwrap();
        let mut next = Vec::with_capacity(prev.len());
        for i in 0..prev.len() {
            if i == 0 {
                next.push(prev[i]);
            } else {
                next.push((prev[i] + prev[i - 1]) / 2.0);
            }
        }
        series.push(next);
    }
    let last_levels: Vec<f64> = (1..=levels).map(|lvl| series[lvl][n - 1]).collect();
    let highest = last_levels
        .iter()
        .cloned()
        .fold(f64::NEG_INFINITY, f64::max);
    let lowest = last_levels.iter().cloned().fold(f64::INFINITY, f64::min);
    let width = highest - lowest;
    let center = last_levels.iter().sum::<f64>() / levels as f64;
    let width_pct = if center.abs() > 1e-12 {
        width / center
    } else {
        0.0
    };
    let label = if width_pct > 0.02 {
        "STRONG_TREND"
    } else if width_pct > 0.005 {
        "TRENDING"
    } else {
        "CONSOLIDATING"
    };
    RainbowSnapshot {
        symbol: sym,
        as_of: as_of.into(),
        bars_used: n,
        levels,
        highest_level: highest,
        lowest_level: lowest,
        rainbow_width: width,
        rainbow_width_pct: width_pct,
        center_value: center,
        r1: last_levels[0],
        r5: last_levels[4],
        r10: last_levels[9],
        last_close: closes[n - 1],
        rainbow_label: label.into(),
        note: String::new(),
    }
}

pub fn compute_mesa_sine_snapshot(
    symbol: &str,
    as_of: &str,
    bars: &[HistoricalPriceRow],
) -> MesaSineSnapshot {
    let sym = symbol.to_uppercase();
    let mut sorted: Vec<&HistoricalPriceRow> = bars.iter().collect();
    sorted.sort_by(|a, b| a.date.cmp(&b.date));
    let n = sorted.len();
    if n < 32 {
        return MesaSineSnapshot {
            symbol: sym,
            as_of: as_of.into(),
            mesa_label: "INSUFFICIENT_DATA".into(),
            note: format!("need ≥32 bars, got {}", n),
            ..Default::default()
        };
    }
    let closes: Vec<f64> = sorted.iter().map(|b| b.close).collect();
    let mut smooth = vec![0.0_f64; n];
    for i in 3..n {
        smooth[i] =
            (4.0 * closes[i] + 3.0 * closes[i - 1] + 2.0 * closes[i - 2] + closes[i - 3]) / 10.0;
    }
    let detrender = |i: usize, src: &[f64]| -> f64 {
        if i < 6 {
            return 0.0;
        }
        (0.0962 * src[i] + 0.5769 * src[i - 2] - 0.5769 * src[i - 4] - 0.0962 * src[i - 6]) * 0.85
    };
    let mut dt = vec![0.0_f64; n];
    for i in 6..n {
        dt[i] = detrender(i, &smooth);
    }
    let mut q1 = vec![0.0_f64; n];
    let mut i1 = vec![0.0_f64; n];
    for i in 6..n {
        q1[i] = detrender(i, &dt);
        i1[i] = if i >= 3 { dt[i - 3] } else { 0.0 };
    }
    let last = n - 1;
    let prev = n - 2;
    let phase_of = |i: usize| -> f64 {
        if i1[i].abs() < 1e-12 {
            0.0
        } else {
            (q1[i] / i1[i]).atan()
        }
    };
    let period = 20.0_f64;
    let phase_now = phase_of(last);
    let phase_prev = phase_of(prev);
    let sine = phase_now.sin();
    let lead = (phase_now + std::f64::consts::FRAC_PI_4).sin();
    let sine_prev = phase_prev.sin();
    let lead_prev = (phase_prev + std::f64::consts::FRAC_PI_4).sin();
    let separation = (sine - lead).abs();
    let crossed_up = sine_prev < lead_prev && sine > lead;
    let crossed_dn = sine_prev > lead_prev && sine < lead;
    let label = if crossed_up {
        "CYCLE_BUY"
    } else if crossed_dn {
        "CYCLE_SELL"
    } else if separation > 0.6 {
        "TRENDING"
    } else {
        "NEUTRAL"
    };
    MesaSineSnapshot {
        symbol: sym,
        as_of: as_of.into(),
        bars_used: n,
        period,
        phase_rad: phase_now,
        sine_value: sine,
        lead_sine: lead,
        sine_prev,
        lead_prev,
        last_close: closes[n - 1],
        mesa_label: label.into(),
        note: String::new(),
    }
}

pub fn compute_frama_snapshot(
    symbol: &str,
    as_of: &str,
    bars: &[HistoricalPriceRow],
) -> FramaSnapshot {
    let sym = symbol.to_uppercase();
    let mut sorted: Vec<&HistoricalPriceRow> = bars.iter().collect();
    sorted.sort_by(|a, b| a.date.cmp(&b.date));
    let n = sorted.len();
    let length = 16usize;
    if n < length * 2 {
        return FramaSnapshot {
            symbol: sym,
            as_of: as_of.into(),
            length,
            frama_label: "INSUFFICIENT_DATA".into(),
            note: format!("need ≥{} bars, got {}", length * 2, n),
            ..Default::default()
        };
    }
    let dim_at = |end_idx: usize| -> f64 {
        let half = length / 2;
        let mut h1 = f64::NEG_INFINITY;
        let mut l1 = f64::INFINITY;
        for k in 0..half {
            let b = sorted[end_idx - k];
            if b.high > h1 {
                h1 = b.high;
            }
            if b.low < l1 {
                l1 = b.low;
            }
        }
        let mut h2 = f64::NEG_INFINITY;
        let mut l2 = f64::INFINITY;
        for k in half..length {
            let b = sorted[end_idx - k];
            if b.high > h2 {
                h2 = b.high;
            }
            if b.low < l2 {
                l2 = b.low;
            }
        }
        let mut h = f64::NEG_INFINITY;
        let mut l = f64::INFINITY;
        for k in 0..length {
            let b = sorted[end_idx - k];
            if b.high > h {
                h = b.high;
            }
            if b.low < l {
                l = b.low;
            }
        }
        let n1 = (h1 - l1) / half as f64;
        let n2 = (h2 - l2) / half as f64;
        let n3 = (h - l) / length as f64;
        if n1 + n2 > 1e-12 && n3 > 1e-12 {
            ((n1 + n2).ln() - n3.ln()) / 2f64.ln()
        } else {
            1.5
        }
    };
    let mut frama_prev_val = sorted[length - 1].close;
    for i in length..n - 1 {
        let d = dim_at(i).clamp(1.0, 2.0);
        let a = (-4.6 * (d - 1.0)).exp().clamp(0.01, 1.0);
        frama_prev_val = a * sorted[i].close + (1.0 - a) * frama_prev_val;
    }
    let d_now = dim_at(n - 1).clamp(1.0, 2.0);
    let a_now = (-4.6 * (d_now - 1.0)).exp().clamp(0.01, 1.0);
    let frama_now = a_now * sorted[n - 1].close + (1.0 - a_now) * frama_prev_val;
    let close = sorted[n - 1].close;
    let spread = close - frama_now;
    let label = if d_now < 1.35 {
        "STRONG_TREND"
    } else if d_now < 1.65 {
        "TREND"
    } else {
        "CHOP"
    };
    FramaSnapshot {
        symbol: sym,
        as_of: as_of.into(),
        bars_used: n,
        length,
        fractal_dim: d_now,
        alpha: a_now,
        frama_value: frama_now,
        frama_prev: frama_prev_val,
        spread,
        last_close: close,
        frama_label: label.into(),
        note: String::new(),
    }
}

pub fn compute_ibs_snapshot(symbol: &str, as_of: &str, bars: &[HistoricalPriceRow]) -> IbsSnapshot {
    let sym = symbol.to_uppercase();
    let mut sorted: Vec<&HistoricalPriceRow> = bars.iter().collect();
    sorted.sort_by(|a, b| a.date.cmp(&b.date));
    let n = sorted.len();
    let length = 14usize;
    if n < length + 1 {
        return IbsSnapshot {
            symbol: sym,
            as_of: as_of.into(),
            length,
            ibs_label: "INSUFFICIENT_DATA".into(),
            note: format!("need ≥{} bars, got {}", length + 1, n),
            ..Default::default()
        };
    }
    let ibs_of = |b: &HistoricalPriceRow| -> f64 {
        let rng = b.high - b.low;
        if rng.abs() < 1e-12 {
            0.5
        } else {
            ((b.close - b.low) / rng).clamp(0.0, 1.0)
        }
    };
    let last = sorted[n - 1];
    let prev = sorted[n - 2];
    let ibs_raw = ibs_of(last);
    let ibs_prev = ibs_of(prev);
    let mut sum = 0.0_f64;
    for k in 0..length {
        sum += ibs_of(sorted[n - 1 - k]);
    }
    let ibs_smoothed = sum / length as f64;
    let label = if ibs_smoothed > 0.8 {
        "OVERBOUGHT"
    } else if ibs_smoothed > 0.6 {
        "BULL"
    } else if ibs_smoothed < 0.2 {
        "OVERSOLD"
    } else if ibs_smoothed < 0.4 {
        "BEAR"
    } else {
        "NEUTRAL"
    };
    IbsSnapshot {
        symbol: sym,
        as_of: as_of.into(),
        bars_used: n,
        length,
        ibs_raw,
        ibs_smoothed,
        ibs_prev,
        last_high: last.high,
        last_low: last.low,
        last_close: last.close,
        ibs_label: label.into(),
        note: String::new(),
    }
}
