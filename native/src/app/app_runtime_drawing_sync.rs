use super::*;

impl TyphooNApp {
    pub(super) fn sync_cross_timeframe_drawings(&mut self) {
        // ── Cross-TF drawing sync ────────────────────────────────────────
        // When drawings_cross_tf is enabled, sync price-based drawings (HLine, FiboRetrace)
        // to all charts with the same symbol. Only syncs HLines (price-only, TF-independent).
        if self.drawings_cross_tf && self.charts.len() > 1 {
            let active = self.active_tab;
            if let Some(src) = self.charts.get(active) {
                let src_sym = src
                    .symbol
                    .split(':')
                    .next()
                    .unwrap_or(&src.symbol)
                    .to_uppercase();
                let src_drawings = src.drawings.clone();
                let src_styles = src.drawing_styles.clone();
                for (i, chart) in self.charts.iter_mut().enumerate() {
                    if i == active {
                        continue;
                    }
                    let chart_sym = chart
                        .symbol
                        .split(':')
                        .next()
                        .unwrap_or(&chart.symbol)
                        .to_uppercase();
                    if chart_sym != src_sym {
                        continue;
                    }
                    // Sync HLines (price-only drawings are TF-independent) + O(1) for Fibo/VLine beyond HLine
                    for (di, d) in src_drawings.iter().enumerate() {
                        if let Drawing::HLine { price, color } = d {
                            let key = format!("{:.10}", price);
                            let already = chart.hline_set.contains(&key);
                            if !already {
                                chart.hline_set.insert(key);
                                chart.drawings.push(Drawing::HLine {
                                    price: *price,
                                    color: *color,
                                });
                                if di < src_styles.len() {
                                    chart.drawing_styles.push(src_styles[di]);
                                }
                            }
                        } else if let Drawing::FiboRetrace {
                            high,
                            low,
                            bar_start,
                            bar_end,
                        } = d
                        {
                            let key = format!("{:.2}-{:.2}-{}-{}", high, low, bar_start, bar_end);
                            let already = chart.fibo_set.contains(&key);
                            if !already {
                                chart.fibo_set.insert(key);
                                chart.drawings.push(Drawing::FiboRetrace {
                                    high: *high,
                                    low: *low,
                                    bar_start: *bar_start,
                                    bar_end: *bar_end,
                                });
                                if di < src_styles.len() {
                                    chart.drawing_styles.push(src_styles[di]);
                                }
                            }
                        } else if let Drawing::VLine { bar_idx, color } = d {
                            let key = bar_idx.to_string();
                            let already = chart.vline_set.contains(&key);
                            if !already {
                                chart.vline_set.insert(key);
                                chart.drawings.push(Drawing::VLine {
                                    bar_idx: *bar_idx,
                                    color: *color,
                                });
                                if di < src_styles.len() {
                                    chart.drawing_styles.push(src_styles[di]);
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}
