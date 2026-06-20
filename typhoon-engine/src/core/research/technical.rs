use super::{HistoricalPriceRow, TechnicalIndicator, TechnicalSnapshot};

// ── TECH compute (technical indicators) ────────────────

/// Compute standard technical indicators (RSI, MACD, Bollinger, ATR, ADX,
/// Stochastic) from a chronologically-ordered slice of bars. Pure compute.
pub fn compute_technical_indicators(
    symbol: &str,
    as_of: &str,
    bars_oldest_first: &[HistoricalPriceRow],
) -> TechnicalSnapshot {
    if bars_oldest_first.len() < 35 {
        return TechnicalSnapshot {
            symbol: symbol.to_uppercase(),
            as_of: as_of.to_string(),
            note: "insufficient bar history (need ≥ 35 bars)".to_string(),
            ..Default::default()
        };
    }
    let n = bars_oldest_first.len();
    let closes: Vec<f64> = bars_oldest_first
        .iter()
        .map(|b| {
            if b.adj_close > 0.0 {
                b.adj_close
            } else {
                b.close
            }
        })
        .collect();
    let highs: Vec<f64> = bars_oldest_first
        .iter()
        .map(|b| b.high.max(b.close))
        .collect();
    let lows: Vec<f64> = bars_oldest_first
        .iter()
        .map(|b| b.low.min(b.close))
        .collect();
    let last_close = closes[n - 1];

    let mut out: Vec<TechnicalIndicator> = Vec::new();

    // RSI(14) — Wilder's smoothing.
    if n >= 15 {
        let mut gains: Vec<f64> = Vec::with_capacity(n - 1);
        let mut losses: Vec<f64> = Vec::with_capacity(n - 1);
        for i in 1..n {
            let diff = closes[i] - closes[i - 1];
            gains.push(if diff > 0.0 { diff } else { 0.0 });
            losses.push(if diff < 0.0 { -diff } else { 0.0 });
        }
        let mut avg_gain: f64 = gains[..14].iter().sum::<f64>() / 14.0;
        let mut avg_loss: f64 = losses[..14].iter().sum::<f64>() / 14.0;
        for i in 14..gains.len() {
            avg_gain = (avg_gain * 13.0 + gains[i]) / 14.0;
            avg_loss = (avg_loss * 13.0 + losses[i]) / 14.0;
        }
        let rs = if avg_loss > 1e-12 {
            avg_gain / avg_loss
        } else {
            f64::INFINITY
        };
        let rsi = if rs.is_infinite() {
            100.0
        } else {
            100.0 - 100.0 / (1.0 + rs)
        };
        let signal = if rsi >= 70.0 {
            "overbought"
        } else if rsi <= 30.0 {
            "oversold"
        } else if rsi >= 55.0 {
            "bullish"
        } else if rsi <= 45.0 {
            "bearish"
        } else {
            "neutral"
        };
        out.push(TechnicalIndicator {
            name: "RSI(14)".to_string(),
            value: rsi,
            value_secondary: 0.0,
            value_tertiary: 0.0,
            signal: signal.to_string(),
            note: String::new(),
        });
    }

    // MACD(12,26,9) — EMA crossover.
    if n >= 35 {
        let ema = |period: usize, data: &[f64]| -> Vec<f64> {
            let k = 2.0 / (period as f64 + 1.0);
            let mut out = Vec::with_capacity(data.len());
            let mut prev = data[0];
            out.push(prev);
            for v in &data[1..] {
                prev = v * k + prev * (1.0 - k);
                out.push(prev);
            }
            out
        };
        let ema12 = ema(12, &closes);
        let ema26 = ema(26, &closes);
        let macd_line: Vec<f64> = ema12.iter().zip(ema26.iter()).map(|(a, b)| a - b).collect();
        let signal_line = ema(9, &macd_line);
        let macd = *macd_line.last().unwrap_or(&0.0);
        let sig = *signal_line.last().unwrap_or(&0.0);
        let hist = macd - sig;
        let signal = if hist > 0.0 {
            "bullish"
        } else if hist < 0.0 {
            "bearish"
        } else {
            "neutral"
        };
        out.push(TechnicalIndicator {
            name: "MACD(12,26,9)".to_string(),
            value: hist,
            value_secondary: macd,
            value_tertiary: sig,
            signal: signal.to_string(),
            note: format!("MACD={:.3} Signal={:.3}", macd, sig),
        });
    }

    // Bollinger Bands (20, 2σ).
    if n >= 20 {
        let slice = &closes[n - 20..];
        let mean: f64 = slice.iter().sum::<f64>() / 20.0;
        let var: f64 = slice.iter().map(|c| (c - mean).powi(2)).sum::<f64>() / 20.0;
        let sd = var.sqrt();
        let upper = mean + 2.0 * sd;
        let lower = mean - 2.0 * sd;
        let bandwidth_pct = if mean > 0.0 {
            (upper - lower) / mean * 100.0
        } else {
            0.0
        };
        let pct_b = if (upper - lower).abs() > 1e-9 {
            (last_close - lower) / (upper - lower) * 100.0
        } else {
            50.0
        };
        let signal = if pct_b >= 100.0 {
            "overbought"
        } else if pct_b <= 0.0 {
            "oversold"
        } else if pct_b >= 80.0 {
            "bullish"
        } else if pct_b <= 20.0 {
            "bearish"
        } else {
            "neutral"
        };
        out.push(TechnicalIndicator {
            name: "BB(20,2)".to_string(),
            value: pct_b,
            value_secondary: upper,
            value_tertiary: lower,
            signal: signal.to_string(),
            note: format!("mid={:.2} bw={:.2}%", mean, bandwidth_pct),
        });
    }

    // ATR(14) — Wilder.
    if n >= 15 {
        let mut tr: Vec<f64> = Vec::with_capacity(n - 1);
        for i in 1..n {
            let hl = highs[i] - lows[i];
            let hc = (highs[i] - closes[i - 1]).abs();
            let lc = (lows[i] - closes[i - 1]).abs();
            tr.push(hl.max(hc).max(lc));
        }
        let mut atr: f64 = tr[..14].iter().sum::<f64>() / 14.0;
        for v in &tr[14..] {
            atr = (atr * 13.0 + v) / 14.0;
        }
        let atr_pct = if last_close > 0.0 {
            atr / last_close * 100.0
        } else {
            0.0
        };
        out.push(TechnicalIndicator {
            name: "ATR(14)".to_string(),
            value: atr,
            value_secondary: atr_pct,
            value_tertiary: 0.0,
            signal: "neutral".to_string(),
            note: format!("{:.2}% of close", atr_pct),
        });
    }

    // ADX(14) — Wilder directional movement.
    if n >= 28 {
        let mut plus_dm: Vec<f64> = Vec::with_capacity(n - 1);
        let mut minus_dm: Vec<f64> = Vec::with_capacity(n - 1);
        let mut tr: Vec<f64> = Vec::with_capacity(n - 1);
        for i in 1..n {
            let up = highs[i] - highs[i - 1];
            let down = lows[i - 1] - lows[i];
            plus_dm.push(if up > down && up > 0.0 { up } else { 0.0 });
            minus_dm.push(if down > up && down > 0.0 { down } else { 0.0 });
            let hl = highs[i] - lows[i];
            let hc = (highs[i] - closes[i - 1]).abs();
            let lc = (lows[i] - closes[i - 1]).abs();
            tr.push(hl.max(hc).max(lc));
        }
        // Wilder smoothing (14).
        let mut pdm: f64 = plus_dm[..14].iter().sum::<f64>();
        let mut mdm: f64 = minus_dm[..14].iter().sum::<f64>();
        let mut trs: f64 = tr[..14].iter().sum::<f64>();
        let mut dx_hist: Vec<f64> = Vec::new();
        for i in 14..plus_dm.len() {
            pdm = pdm - pdm / 14.0 + plus_dm[i];
            mdm = mdm - mdm / 14.0 + minus_dm[i];
            trs = trs - trs / 14.0 + tr[i];
            let plus_di = if trs > 1e-12 { pdm / trs * 100.0 } else { 0.0 };
            let minus_di = if trs > 1e-12 { mdm / trs * 100.0 } else { 0.0 };
            let sum = plus_di + minus_di;
            let dx = if sum > 1e-12 {
                ((plus_di - minus_di).abs() / sum) * 100.0
            } else {
                0.0
            };
            dx_hist.push(dx);
        }
        if dx_hist.len() >= 14 {
            let mut adx: f64 = dx_hist[..14].iter().sum::<f64>() / 14.0;
            for v in &dx_hist[14..] {
                adx = (adx * 13.0 + v) / 14.0;
            }
            let plus_di = if trs > 1e-12 { pdm / trs * 100.0 } else { 0.0 };
            let minus_di = if trs > 1e-12 { mdm / trs * 100.0 } else { 0.0 };
            let signal = if adx >= 25.0 {
                if plus_di > minus_di {
                    "bullish"
                } else {
                    "bearish"
                }
            } else {
                "neutral"
            };
            out.push(TechnicalIndicator {
                name: "ADX(14)".to_string(),
                value: adx,
                value_secondary: plus_di,
                value_tertiary: minus_di,
                signal: signal.to_string(),
                note: format!("+DI={:.1} −DI={:.1}", plus_di, minus_di),
            });
        }
    }

    // Stochastic %K(14), %D(3).
    if n >= 17 {
        let mut k_series: Vec<f64> = Vec::new();
        for i in 13..n {
            let window_high = highs[i - 13..=i]
                .iter()
                .cloned()
                .fold(f64::NEG_INFINITY, f64::max);
            let window_low = lows[i - 13..=i]
                .iter()
                .cloned()
                .fold(f64::INFINITY, f64::min);
            let denom = window_high - window_low;
            let k = if denom.abs() > 1e-12 {
                (closes[i] - window_low) / denom * 100.0
            } else {
                50.0
            };
            k_series.push(k);
        }
        let k_last = *k_series.last().unwrap_or(&50.0);
        let d_last = if k_series.len() >= 3 {
            k_series[k_series.len() - 3..].iter().sum::<f64>() / 3.0
        } else {
            k_last
        };
        let signal = if k_last >= 80.0 {
            "overbought"
        } else if k_last <= 20.0 {
            "oversold"
        } else if k_last > d_last {
            "bullish"
        } else if k_last < d_last {
            "bearish"
        } else {
            "neutral"
        };
        out.push(TechnicalIndicator {
            name: "Stoch(14,3)".to_string(),
            value: k_last,
            value_secondary: d_last,
            value_tertiary: 0.0,
            signal: signal.to_string(),
            note: format!("%K={:.1} %D={:.1}", k_last, d_last),
        });
    }

    // Trend synthesis — count bullish/bearish across tradeable indicators.
    let mut bull = 0usize;
    let mut bear = 0usize;
    for ind in &out {
        match ind.signal.as_str() {
            "bullish" | "overbought" => bull += 1,
            "bearish" | "oversold" => bear += 1,
            _ => {}
        }
    }
    let trend_summary = if bull > bear + 1 {
        "bullish composite".to_string()
    } else if bear > bull + 1 {
        "bearish composite".to_string()
    } else {
        "mixed / neutral composite".to_string()
    };

    TechnicalSnapshot {
        symbol: symbol.to_uppercase(),
        as_of: as_of.to_string(),
        last_close,
        indicators: out,
        trend_summary,
        note: String::new(),
    }
}
