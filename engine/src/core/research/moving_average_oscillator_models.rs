use super::*;

mod regression_pivot_candles;
pub use regression_pivot_candles::*;
mod adaptive_forecast_vigor;
pub use adaptive_forecast_vigor::*;
mod adaptive_volume_momentum;
pub use adaptive_volume_momentum::*;
mod acceleration_range_impulse;
pub use acceleration_range_impulse::*;
mod momentum_envelope_volume;
pub use momentum_envelope_volume::*;
mod adaptive_cycle_volume;
pub use adaptive_cycle_volume::*;

// Shared moving-average helpers used by oscillator model families.

pub(super) fn ema_series(values: &[f64], length: usize) -> Vec<f64> {
    let n = values.len();
    if n == 0 || length == 0 {
        return Vec::new();
    }
    let alpha = 2.0 / (length as f64 + 1.0);
    let mut out = Vec::with_capacity(n);
    out.push(values[0]);
    for i in 1..n {
        out.push(alpha * values[i] + (1.0 - alpha) * out[i - 1]);
    }
    out
}

// Shared simple moving-average helper used by oscillator model families.

pub(super) fn sma_series(values: &[f64], length: usize) -> Vec<f64> {
    let n = values.len();
    let mut out = vec![0.0; n];
    if n == 0 || length == 0 {
        return out;
    }
    let mut acc = 0.0;
    for i in 0..n {
        acc += values[i];
        if i >= length {
            acc -= values[i - length];
        }
        out[i] = if i + 1 >= length {
            acc / length as f64
        } else {
            0.0
        };
    }
    out
}
