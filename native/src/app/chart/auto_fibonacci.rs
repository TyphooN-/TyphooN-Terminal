use super::*;

impl ChartState {
    /// Compute Auto Fibonacci levels from fractal swing points.
    /// Mirrors AutoFibonacci.mqh: finds most significant recent swing high/low
    /// and computes retracement (0-100%) + extension (127.2-423.6%) levels.
    pub(crate) fn compute_auto_fibonacci(&mut self) {
        self.auto_fib_levels.clear();
        self.auto_fib_swing = None;
        if self.bars.len() < 20 {
            return;
        }

        let lookback = 10usize; // InpFractalLookback
        let recent_start = (self.bars.len() as f64 * 0.4) as usize; // search recent 60%
        let search = &self.bars[recent_start..];

        // Find swing high and swing low from fractals in search range
        let mut swing_high: Option<(f64, usize)> = None;
        let mut swing_low: Option<(f64, usize)> = None;

        for i in lookback..search.len().saturating_sub(lookback) {
            let abs_i = recent_start + i;
            if abs_i < self.fractal_up.len() && self.fractal_up[abs_i] {
                if swing_high.map_or(true, |(h, _)| search[i].high > h) {
                    swing_high = Some((search[i].high, abs_i));
                }
            }
            if abs_i < self.fractal_down.len() && self.fractal_down[abs_i] {
                if swing_low.map_or(true, |(l, _)| search[i].low < l) {
                    swing_low = Some((search[i].low, abs_i));
                }
            }
        }

        if let (Some((high, hi_idx)), Some((low, lo_idx))) = (swing_high, swing_low) {
            if (high - low).abs() < f64::EPSILON {
                return;
            }
            self.auto_fib_swing = Some((high, low, hi_idx, lo_idx));
            let range = high - low;
            let is_bull = lo_idx < hi_idx; // uptrend: low comes before high

            // Retracement levels (from high toward low for bull, from low toward high for bear)
            let retrace_levels = [0.0, 0.236, 0.382, 0.5, 0.618, 0.786, 1.0];
            let retrace_labels = ["0%", "23.6%", "38.2%", "50%", "61.8%", "78.6%", "100%"];
            for (lvl, label) in retrace_levels.iter().zip(retrace_labels.iter()) {
                let price = if is_bull {
                    high - lvl * range
                } else {
                    low + lvl * range
                };
                self.auto_fib_levels.push((price, label.to_string(), false));
            }

            // Extension levels (beyond the swing)
            let ext_levels = [1.272, 1.618, 2.0, 2.618, 3.618, 4.236];
            let ext_labels = ["127.2%", "161.8%", "200%", "261.8%", "361.8%", "423.6%"];
            for (lvl, label) in ext_levels.iter().zip(ext_labels.iter()) {
                let price = if is_bull {
                    low + lvl * range
                } else {
                    high - lvl * range
                };
                self.auto_fib_levels.push((price, label.to_string(), true));
            }
        }
    }
}
