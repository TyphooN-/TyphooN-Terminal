pub(super) fn compute_prev_candle_levels(
    bars: &[Bar],
) -> (
    (Option<f64>, Option<f64>), // H1
    (Option<f64>, Option<f64>), // H4
    (Option<f64>, Option<f64>), // D1
    (Option<f64>, Option<f64>), // W1
    (Option<f64>, Option<f64>), // MN1
) {
    if bars.len() < 2 {
        return (
            (None, None),
            (None, None),
            (None, None),
            (None, None),
            (None, None),
        );
    }

    fn group_prev(bars: &[Bar], period_ms: i64) -> (Option<f64>, Option<f64>) {
        let mut groups: Vec<(f64, f64)> = Vec::new();
        let mut current_period = -1_i64;
        let mut hi = f64::MIN;
        let mut lo = f64::MAX;
        for bar in bars {
            let p = bar.ts_ms / period_ms;
            if p != current_period {
                if current_period >= 0 {
                    groups.push((hi, lo));
                }
                current_period = p;
                hi = bar.high;
                lo = bar.low;
            } else {
                hi = hi.max(bar.high);
                lo = lo.min(bar.low);
            }
        }
        if current_period >= 0 {
            groups.push((hi, lo));
        }
        if groups.len() >= 2 {
            let prev = &groups[groups.len() - 2];
            (Some(prev.0), Some(prev.1))
        } else {
            (None, None)
        }
    }

    // Weekly: group by ISO week (Monday start) — fixes the old 7-day epoch bucket
    fn group_prev_weekly(bars: &[Bar]) -> (Option<f64>, Option<f64>) {
        let mut groups: Vec<(f64, f64)> = Vec::new();
        let mut cur_week = 0i32;
        let mut hi = f64::MIN;
        let mut lo = f64::MAX;
        for bar in bars {
            let dt = chrono::DateTime::from_timestamp_millis(bar.ts_ms).unwrap_or_default();
            use chrono::Datelike;
            // ISO week key: year * 100 + week number (Monday start)
            let week_key = dt.year() * 100 + dt.iso_week().week() as i32;
            if week_key != cur_week {
                if cur_week > 0 {
                    groups.push((hi, lo));
                }
                cur_week = week_key;
                hi = bar.high;
                lo = bar.low;
            } else {
                hi = hi.max(bar.high);
                lo = lo.min(bar.low);
            }
        }
        if cur_week > 0 {
            groups.push((hi, lo));
        }
        if groups.len() >= 2 {
            let prev = &groups[groups.len() - 2];
            (Some(prev.0), Some(prev.1))
        } else {
            (None, None)
        }
    }

    // Monthly: group by year-month
    fn group_prev_monthly(bars: &[Bar]) -> (Option<f64>, Option<f64>) {
        let mut groups: Vec<(f64, f64)> = Vec::new();
        let mut cur_ym = 0i32;
        let mut hi = f64::MIN;
        let mut lo = f64::MAX;
        for bar in bars {
            let dt = chrono::DateTime::from_timestamp_millis(bar.ts_ms).unwrap_or_default();
            use chrono::Datelike;
            let ym = dt.year() * 100 + dt.month() as i32;
            if ym != cur_ym {
                if cur_ym > 0 {
                    groups.push((hi, lo));
                }
                cur_ym = ym;
                hi = bar.high;
                lo = bar.low;
            } else {
                hi = hi.max(bar.high);
                lo = lo.min(bar.low);
            }
        }
        if cur_ym > 0 {
            groups.push((hi, lo));
        }
        if groups.len() >= 2 {
            let prev = &groups[groups.len() - 2];
            (Some(prev.0), Some(prev.1))
        } else {
            (None, None)
        }
    }

    let h1 = group_prev(bars, 3_600_000); // 1 hour
    let h4 = group_prev(bars, 14_400_000); // 4 hours
    let d1 = group_prev(bars, 86_400_000); // 1 day
    let w1 = group_prev_weekly(bars);     // proper Monday-aligned week
    let mn1 = group_prev_monthly(bars);

    (h1, h4, d1, w1, mn1)
}