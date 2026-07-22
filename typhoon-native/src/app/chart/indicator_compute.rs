use super::*;

/// GPU/CPU indicator computation for a chart viewport (ADR-125 Target 2). A native
/// extension trait, not an inherent impl, because it drives the native `gpu_compute`
/// (wgpu) pipeline which stays in `typhoon-native`; `ChartState` itself lives in
/// `typhoon-chart-ui`. Re-exported from `chart` so call sites keep `chart.compute_indicators(…)`.
pub(crate) trait ChartIndicatorCompute {
    fn compute_indicators(&mut self);
    fn compute_indicators_gpu(&mut self, gpu: Option<&mut gpu_compute::GpuCompute>);
}

impl ChartIndicatorCompute for ChartState {
    fn compute_indicators(&mut self) {
        self.compute_indicators_gpu(None);
    }

    fn compute_indicators_gpu(&mut self, gpu: Option<&mut gpu_compute::GpuCompute>) {
        let n = self.bars.len();
        let forming_bar_dirty_at_entry = self.forming_bar_dirty;
        // Cache reloads replace `bars` with the last persisted candle, which can lag the
        // live quote already shown in the watchlist/position panels. Fold the fresh live
        // mid back into the last bar before either the incremental or full GPU path so a
        // reload cannot make the active forming candle jump backward until the next tick.
        let live_quote_folded = self.fold_fresh_live_quote_into_forming_bar();
        if live_quote_folded && !forming_bar_dirty_at_entry {
            self.forming_bar_dirty = false;
        }

        // Forming-bar fast path: only update the last value of indicators
        // instead of full recompute + GPU upload. This is the key integration
        // point between our WS fast-path and the GPU compute path.
        // O(1) path for SMA/EMA (with hoisted close); stateful indicators
        // (KAMA, RSI, MACD, ATR, ...) intentionally fall through to the next
        // structural change (new closed bar) for full GPU dispatch.
        if forming_bar_dirty_at_entry && n > 1 {
            if let Some(last) = self.bars.last_mut() {
                let mut close = last.close;

                // When live quotes are present, fold the live mid into the forming bar so
                // the candle grows with real-time data (prevents the stale/grey candle).
                // Delayed quotes (iapi equities) are excluded: folding a stale delayed
                // mid would fight the consolidated last the watchlist already folds in,
                // decoupling the candle from the watchlist (see `fresh_live_quote_mid`).
                let has_live_quotes =
                    !self.live_quote_delayed && self.live_bid > 0.0 && self.live_ask > 0.0;
                if has_live_quotes {
                    let mid = (self.live_bid + self.live_ask) * 0.5;
                    last.close = mid;
                    last.high = last.high.max(mid);
                    last.low = last.low.min(mid);
                    close = mid;
                }

                if let Some(gpu) = gpu {
                    let is_live = if has_live_quotes { 1.0 } else { 0.0 };
                    gpu.upload_forming_bar(
                        last.open as f32,
                        last.high as f32,
                        last.low as f32,
                        close as f32,
                        last.volume as f32,
                        is_live,
                    );
                }

                // Indicator rolling updates still happen (they only need the close value)
                // For SMA200 / SMA100 we can do a cheap rolling update
                let prev200 = self.sma200.get(n - 2).copied().flatten();
                if let (Some(last_sma200), Some(prev)) = (self.sma200.last_mut(), prev200) {
                    *last_sma200 = Some(
                        (prev * (self.sma_slow_period as f64 - 1.0) + close)
                            / self.sma_slow_period as f64,
                    );
                }
                let prev100 = self.sma100.get(n - 2).copied().flatten();
                if let (Some(last_sma100), Some(prev)) = (self.sma100.last_mut(), prev100) {
                    *last_sma100 = Some(
                        (prev * (self.sma_fast_period as f64 - 1.0) + close)
                            / self.sma_fast_period as f64,
                    );
                }
                // EMA21 fast-path last-value update (O(1) rolling)
                let ema_p = self.ema_period as f64;
                let k = 2.0 / (ema_p + 1.0);
                let prev_ema = self.ema21.get(n - 2).copied().flatten();
                if let (Some(last_ema), Some(prev)) = (self.ema21.last_mut(), prev_ema) {
                    *last_ema = Some(close * k + prev * (1.0 - k));
                }
            }
            self.forming_bar_dirty = false; // consumed
            return;
        }

        self.bars_prev_daily_close = if matches!(
            self.timeframe,
            Timeframe::M1
                | Timeframe::M5
                | Timeframe::M15
                | Timeframe::M30
                | Timeframe::H1
                | Timeframe::H4
                | Timeframe::D1
        ) {
            self.bars.last().and_then(|last| {
                let latest_day = last.ts_ms / 86_400_000;
                self.bars
                    .iter()
                    .rev()
                    .find(|bar| bar.ts_ms / 86_400_000 < latest_day)
                    .map(|bar| bar.close)
            })
        } else {
            None
        }
        .unwrap_or(0.0);

        // ── GPU path: upload bars to VRAM, compute on GPU, read back ──
        if let Some(gpu) = gpu {
            if n > 0 {
                // Reuse upload buffers to avoid repeated allocations
                if self.upload_opens.len() < n {
                    self.upload_opens = Vec::with_capacity(n);
                    self.upload_closes = Vec::with_capacity(n);
                    self.upload_highs = Vec::with_capacity(n);
                    self.upload_lows = Vec::with_capacity(n);
                    self.upload_volumes = Vec::with_capacity(n);
                }
                self.upload_opens.clear();
                self.upload_closes.clear();
                self.upload_highs.clear();
                self.upload_lows.clear();
                self.upload_volumes.clear();
                for b in &self.bars {
                    self.upload_opens.push(b.open as f32);
                    self.upload_closes.push(b.close as f32);
                    self.upload_highs.push(b.high as f32);
                    self.upload_lows.push(b.low as f32);
                    self.upload_volumes.push(b.volume as f32);
                }
                gpu.upload_bars_full(
                    &self.upload_opens,
                    &self.upload_closes,
                    &self.upload_highs,
                    &self.upload_lows,
                    &self.upload_volumes,
                );

                // Update snapshot so the draw_chart early-out works correctly after GPU path
                self.last_rendered_gen = self.visible_bars_gen;
                self.last_rendered_bar_ts = self.last_visible_bar_ts;

                // SMA — parallel GPU
                let sma_slow = self.sma_slow_period;
                let sma_fast = self.sma_fast_period;
                // Prefer dedicated compute_sma_gpu when available
                if let Some(data) = gpu.compute_sma_gpu(sma_slow, 0, self.bars.len() as u32) {
                    self.sma200 = data
                        .iter()
                        .map(|&v| if v == 0.0 { None } else { Some(v as f64) })
                        .collect();
                } else if let Some(data) =
                    gpu.dispatch_indicator_pub(&gpu_compute::Indicator::Sma, sma_slow, true)
                {
                    self.sma200 = data
                        .iter()
                        .map(|&v| if v == 0.0 { None } else { Some(v as f64) })
                        .collect();
                } else {
                    self.sma200 = compute_sma(&self.bars, sma_slow as usize);
                }

                if let Some(data) =
                    gpu.dispatch_indicator_pub(&gpu_compute::Indicator::Sma, sma_fast, true)
                {
                    self.sma100 = data
                        .iter()
                        .map(|&v| if v == 0.0 { None } else { Some(v as f64) })
                        .collect();
                } else {
                    self.sma100 = compute_sma(&self.bars, sma_fast as usize);
                }

                // KAMA — sequential GPU
                if let Some(data) = gpu.compute_kama_gpu(10) {
                    self.kama = data
                        .iter()
                        .enumerate()
                        .map(|(i, &v)| if i < 10 { None } else { Some(v as f64) })
                        .collect();
                } else if let Some(data) =
                    gpu.dispatch_indicator_pub(&gpu_compute::Indicator::Kama, 10, false)
                {
                    self.kama = data
                        .iter()
                        .enumerate()
                        .map(|(i, &v)| if i < 10 { None } else { Some(v as f64) })
                        .collect();
                } else {
                    self.kama = compute_kama(&self.bars, 10, 2, 30);
                }

                // EMA — sequential GPU
                let ema_p = self.ema_period;
                if let Some(data) =
                    gpu.dispatch_indicator_pub(&gpu_compute::Indicator::Ema, ema_p, false)
                {
                    self.ema21 = data
                        .iter()
                        .map(|&v| if v == 0.0 { None } else { Some(v as f64) })
                        .collect();
                } else {
                    self.ema21 = compute_ema(&self.bars, ema_p as usize);
                }

                // Bollinger — parallel GPU
                let bb_p = self.bb_period;
                if let Some(data) = gpu.compute_bollinger_gpu(bb_p) {
                    let mut mid = Vec::with_capacity(n);
                    let mut upper = Vec::with_capacity(n);
                    let mut lower = Vec::with_capacity(n);
                    for i in 0..n {
                        let m = data.get(i * 3).copied().unwrap_or(0.0);
                        let u = data.get(i * 3 + 1).copied().unwrap_or(0.0);
                        let l = data.get(i * 3 + 2).copied().unwrap_or(0.0);
                        if m == 0.0 {
                            mid.push(None);
                            upper.push(None);
                            lower.push(None);
                        } else {
                            mid.push(Some(m as f64));
                            upper.push(Some(u as f64));
                            lower.push(Some(l as f64));
                        }
                    }
                    self.bb_mid = mid;
                    self.bb_upper = upper;
                    self.bb_lower = lower;
                } else {
                    let (m, u, l) = compute_bollinger(&self.bars, bb_p as usize, 2.0);
                    self.bb_mid = m;
                    self.bb_upper = u;
                    self.bb_lower = l;
                }

                // RSI — sequential GPU
                let rsi_p = self.rsi_period;
                // Prefer dedicated RSI GPU path, then generic dispatch, then CPU
                if let Some(data) = gpu.compute_rsi_gpu(rsi_p) {
                    self.rsi = data
                        .iter()
                        .enumerate()
                        .map(|(i, &v)| {
                            if i < rsi_p as usize || v == 0.0 {
                                None
                            } else {
                                Some(v as f64)
                            }
                        })
                        .collect();
                } else if let Some(data) =
                    gpu.dispatch_indicator_pub(&gpu_compute::Indicator::Rsi, rsi_p, false)
                {
                    self.rsi = data
                        .iter()
                        .enumerate()
                        .map(|(i, &v)| {
                            if i < rsi_p as usize || v == 0.0 {
                                None
                            } else {
                                Some(v as f64)
                            }
                        })
                        .collect();
                } else {
                    self.rsi = compute_rsi(&self.bars, rsi_p as usize);
                }

                // Fisher — sequential GPU (uses midpoints)
                let fisher_p = self.fisher_period;
                if let Some(data) = gpu.compute_fisher_gpu(fisher_p) {
                    let mut f = Vec::with_capacity(n);
                    let mut fs = Vec::with_capacity(n);
                    for i in 0..n {
                        let fv = data.get(i * 2).copied().unwrap_or(0.0);
                        let sv = data.get(i * 2 + 1).copied().unwrap_or(0.0);
                        if i < fisher_p as usize || (fv == 0.0 && sv == 0.0) {
                            f.push(None);
                            fs.push(None);
                        } else {
                            f.push(Some(fv as f64));
                            fs.push(Some(sv as f64));
                        }
                    }
                    self.fisher = f;
                    self.fisher_signal = fs;
                } else if let Some(data) =
                    gpu.dispatch_indicator_pub(&gpu_compute::Indicator::Fisher, fisher_p, true)
                {
                    let mut f = Vec::with_capacity(n);
                    let mut fs = Vec::with_capacity(n);
                    for i in 0..n {
                        let fv = data.get(i * 2).copied().unwrap_or(0.0);
                        let sv = data.get(i * 2 + 1).copied().unwrap_or(0.0);
                        if i < fisher_p as usize || (fv == 0.0 && sv == 0.0) {
                            f.push(None);
                            fs.push(None);
                        } else {
                            f.push(Some(fv as f64));
                            fs.push(Some(sv as f64));
                        }
                    }
                    self.fisher = f;
                    self.fisher_signal = fs;
                } else {
                    let (f, fs) = compute_fisher(&self.bars, fisher_p as usize);
                    self.fisher = f;
                    self.fisher_signal = fs;
                }

                // ATR — sequential GPU (uses OHLC)
                let atr_p = self.atr_period;
                if let Some(data) = gpu.compute_atr_gpu(atr_p) {
                    self.atr = data
                        .iter()
                        .map(|&v| if v == 0.0 { None } else { Some(v as f64) })
                        .collect();
                } else if let Some(data) =
                    gpu.dispatch_ohlc_indicator_pub(&gpu_compute::Indicator::Atr, atr_p, 1)
                {
                    self.atr = data
                        .iter()
                        .map(|&v| if v == 0.0 { None } else { Some(v as f64) })
                        .collect();
                } else {
                    self.atr = compute_atr(&self.bars, atr_p as usize);
                }

                // MACD — sequential GPU with dynamic periods
                if let Some(data) =
                    gpu.compute_macd_gpu_dynamic(self.macd_fast, self.macd_slow, self.macd_signal_p)
                {
                    // Reuse existing Vec allocations (clear + refill instead of new Vec)
                    self.macd_line.clear();
                    self.macd_signal.clear();
                    self.macd_hist.clear();
                    self.macd_line.reserve(n);
                    self.macd_signal.reserve(n);
                    self.macd_hist.reserve(n);
                    for i in 0..n {
                        let l = data.get(i * 3).copied().unwrap_or(0.0);
                        let s = data.get(i * 3 + 1).copied().unwrap_or(0.0);
                        let h = data.get(i * 3 + 2).copied().unwrap_or(0.0);
                        if i < self.macd_slow as usize || (l == 0.0 && s == 0.0 && h == 0.0) {
                            self.macd_line.push(None);
                            self.macd_signal.push(None);
                            self.macd_hist.push(None);
                        } else {
                            self.macd_line.push(Some(l as f64));
                            self.macd_signal.push(Some(s as f64));
                            self.macd_hist.push(Some(h as f64));
                        }
                    }
                } else {
                    let (ml, ms, mh) = compute_macd(
                        &self.bars,
                        self.macd_fast as usize,
                        self.macd_slow as usize,
                        self.macd_signal_p as usize,
                    );
                    self.macd_line = ml;
                    self.macd_signal = ms;
                    self.macd_hist = mh;
                }

                // Stochastic — sequential GPU (uses OHLC, warmup: period+3 bars)
                let stoch_p = self.stoch_period;
                if let Some(data) = gpu.compute_stochastic_gpu(stoch_p) {
                    let mut sk = Vec::with_capacity(n);
                    let mut sd = Vec::with_capacity(n);
                    for i in 0..n {
                        if i < stoch_p as usize {
                            sk.push(None);
                            sd.push(None);
                        } else {
                            sk.push(Some(data.get(i * 2).copied().unwrap_or(50.0) as f64));
                            sd.push(Some(data.get(i * 2 + 1).copied().unwrap_or(50.0) as f64));
                        }
                    }
                    self.stoch_k = sk;
                    self.stoch_d = sd;
                } else if let Some(data) =
                    gpu.dispatch_indicator_pub(&gpu_compute::Indicator::Stochastic, stoch_p, true)
                {
                    let mut sk = Vec::with_capacity(n);
                    let mut sd = Vec::with_capacity(n);
                    for i in 0..n {
                        if i < stoch_p as usize {
                            sk.push(None);
                            sd.push(None);
                        } else {
                            sk.push(Some(data.get(i * 2).copied().unwrap_or(50.0) as f64));
                            sd.push(Some(data.get(i * 2 + 1).copied().unwrap_or(50.0) as f64));
                        }
                    }
                    self.stoch_k = sk;
                    self.stoch_d = sd;
                } else {
                    let (sk, sd) = compute_stochastic(&self.bars, stoch_p as usize, 3, 3);
                    self.stoch_k = sk;
                    self.stoch_d = sd;
                }

                // ADX — sequential GPU (uses OHLC, warmup: 2×period bars)
                let adx_p = self.adx_period;
                if let Some(data) = gpu.compute_adx_gpu(adx_p) {
                    let mut adx = Vec::with_capacity(n);
                    let mut dip = Vec::with_capacity(n);
                    let mut dim = Vec::with_capacity(n);
                    for i in 0..n {
                        let a = data.get(i * 3).copied().unwrap_or(0.0);
                        let dp = data.get(i * 3 + 1).copied().unwrap_or(0.0);
                        let dm = data.get(i * 3 + 2).copied().unwrap_or(0.0);
                        let di_warmup = adx_p as usize;
                        let adx_warmup = (adx_p as usize * 2).saturating_sub(1);
                        dip.push(if i < di_warmup || (dp == 0.0 && dm == 0.0) {
                            None
                        } else {
                            Some(dp as f64)
                        });
                        dim.push(if i < di_warmup || (dp == 0.0 && dm == 0.0) {
                            None
                        } else {
                            Some(dm as f64)
                        });
                        adx.push(if i < adx_warmup || a == 0.0 {
                            None
                        } else {
                            Some(a as f64)
                        });
                    }
                    self.adx = adx;
                    self.di_plus = dip;
                    self.di_minus = dim;
                } else if let Some(data) =
                    gpu.dispatch_indicator_pub(&gpu_compute::Indicator::Adx, adx_p, true)
                {
                    let mut adx = Vec::with_capacity(n);
                    let mut dip = Vec::with_capacity(n);
                    let mut dim = Vec::with_capacity(n);
                    for i in 0..n {
                        let a = data.get(i * 3).copied().unwrap_or(0.0);
                        let dp = data.get(i * 3 + 1).copied().unwrap_or(0.0);
                        let dm = data.get(i * 3 + 2).copied().unwrap_or(0.0);
                        let di_warmup = adx_p as usize;
                        let adx_warmup = (adx_p as usize * 2).saturating_sub(1);
                        dip.push(if i < di_warmup || (dp == 0.0 && dm == 0.0) {
                            None
                        } else {
                            Some(dp as f64)
                        });
                        dim.push(if i < di_warmup || (dp == 0.0 && dm == 0.0) {
                            None
                        } else {
                            Some(dm as f64)
                        });
                        adx.push(if i < adx_warmup || a == 0.0 {
                            None
                        } else {
                            Some(a as f64)
                        });
                    }
                    self.adx = adx;
                    self.di_plus = dip;
                    self.di_minus = dim;
                } else {
                    let (adx, dip, dim) = compute_adx(&self.bars, adx_p as usize);
                    self.adx = adx;
                    self.di_plus = dip;
                    self.di_minus = dim;
                }

                // Remaining indicators — GPU where shader exists, CPU fallback

                // Ichimoku — GPU (sequential, 4 outputs per bar)
                // Warmup: Tenkan=8, Kijun=25, SpanA=51, SpanB=77 bars
                if let Some(data) = gpu.compute_ichimoku_gpu() {
                    let n = self.bars.len();
                    let mut tk = Vec::with_capacity(n);
                    let mut kj = Vec::with_capacity(n);
                    let mut sa = Vec::with_capacity(n);
                    let mut sb = Vec::with_capacity(n);
                    for i in 0..n {
                        let t = data.get(i * 4).copied().unwrap_or(0.0);
                        let k = data.get(i * 4 + 1).copied().unwrap_or(0.0);
                        let a = data.get(i * 4 + 2).copied().unwrap_or(0.0);
                        let b = data.get(i * 4 + 3).copied().unwrap_or(0.0);
                        tk.push(if i < 9 { None } else { Some(t as f64) });
                        kj.push(if i < 26 { None } else { Some(k as f64) });
                        sa.push(if i < 52 { None } else { Some(a as f64) });
                        sb.push(if i < 52 { None } else { Some(b as f64) });
                    }
                    self.ichi_tenkan = tk;
                    self.ichi_kijun = kj;
                    self.ichi_span_a = sa;
                    self.ichi_span_b = sb;
                } else {
                    let (tk, kj, sa, sb) = compute_ichimoku(&self.bars, 9, 26, 52);
                    self.ichi_tenkan = tk;
                    self.ichi_kijun = kj;
                    self.ichi_span_a = sa;
                    self.ichi_span_b = sb;
                }

                // WMA — GPU (parallel)
                if let Some(data) = gpu.compute_wma_gpu(20) {
                    self.wma = data
                        .iter()
                        .map(|&v| if v == 0.0 { None } else { Some(v as f64) })
                        .collect();
                } else if let Some(data) =
                    gpu.dispatch_indicator_pub(&gpu_compute::Indicator::Wma, 20, false)
                {
                    self.wma = data
                        .iter()
                        .map(|&v| if v == 0.0 { None } else { Some(v as f64) })
                        .collect();
                } else {
                    self.wma = compute_wma(&self.bars, 20);
                }

                // HMA — GPU (WMA composition shader)
                if let Some(data) = gpu.compute_hma_gpu(20) {
                    self.hma = data
                        .iter()
                        .map(|&v| if v == 0.0 { None } else { Some(v as f64) })
                        .collect();
                } else if let Some(data) =
                    gpu.dispatch_indicator_pub(&gpu_compute::Indicator::Hma, 20, false)
                {
                    self.hma = data
                        .iter()
                        .map(|&v| if v == 0.0 { None } else { Some(v as f64) })
                        .collect();
                } else {
                    self.hma = compute_hma(&self.bars, 20);
                }

                // CCI — GPU (parallel, from OHLC, warmup: period-1 bars)
                if let Some(data) = gpu.compute_cci_gpu(20) {
                    self.cci = data
                        .iter()
                        .enumerate()
                        .map(|(i, &v)| if i < 19 { None } else { Some(v as f64) })
                        .collect();
                } else {
                    self.cci = compute_cci(&self.bars, 20);
                }

                // Williams %R — GPU (parallel, from OHLC, first valid at period-1)
                if let Some(data) = gpu.compute_williams_r_gpu(14) {
                    self.williams_r = data
                        .iter()
                        .enumerate()
                        .map(|(i, &v)| if i < 13 { None } else { Some(v as f64) })
                        .collect();
                } else {
                    self.williams_r = compute_williams_r(&self.bars, 14);
                }

                // OBV — GPU (sequential, resident close + volume buffers)
                if let Some(data) = gpu.compute_obv_gpu() {
                    self.obv = data.iter().map(|&v| Some(v as f64)).collect();
                } else {
                    self.obv = compute_obv(&self.bars);
                }

                // Momentum — GPU (parallel, oscillator — 0.0 is valid)
                let mom_p = self.momentum_period;
                if let Some(data) = gpu.compute_momentum_gpu(mom_p) {
                    self.momentum = data
                        .iter()
                        .enumerate()
                        .map(|(i, &v)| {
                            if i < mom_p as usize {
                                None
                            } else {
                                Some(v as f64)
                            }
                        })
                        .collect();
                } else {
                    self.momentum = compute_momentum(&self.bars, mom_p as usize);
                }

                // Simple O(1) forming-bar update for Momentum (approximate)
                if self.forming_bar_dirty && n > 1 && mom_p as usize > 0 {
                    if let Some(prev_mom) = self.momentum.get(n - 2).copied().flatten() {
                        if let Some(last_mom) = self.momentum.last_mut() {
                            if let Some(last) = self.bars.last() {
                                // Approximate: shift by the change in close
                                let change = last.close - self.bars[n - 2].close;
                                *last_mom = Some(prev_mom + change);
                            }
                        }
                    }
                }

                // Simple O(1) forming-bar update for Rate of Change (approximate)
                if self.forming_bar_dirty && n > 1 && mom_p as usize > 0 {
                    if let Some(_prev_roc) = self.momentum.get(n - 2).copied().flatten() {
                        if let Some(last_roc) = self.momentum.last_mut() {
                            if let Some(last) = self.bars.last() {
                                let prev_close = self.bars[n - 2].close;
                                if prev_close != 0.0 {
                                    let new_roc = ((last.close - prev_close) / prev_close) * 100.0;
                                    *last_roc = Some(new_roc);
                                }
                            }
                        }
                    }
                }

                // O(1) forming-bar update for Linear Regression Intercept
                if self.forming_bar_dirty && n > 1 {
                    if let Some(last_slope) = self.linreg_slope.get(n - 2).copied().flatten() {
                        if let Some(last_intercept) = self.linreg_intercept.last_mut() {
                            if let Some(last) = self.bars.last() {
                                // intercept = y - slope * x  (using current bar as reference)
                                let x = (n - 1) as f64;
                                *last_intercept = Some(last.close - last_slope * x);
                            }
                        }
                    }
                }

                // Simple O(1) forming-bar update for Chande Forecast Oscillator (CFO)
                if self.forming_bar_dirty && n > 1 {
                    if let Some(last_slope) = self.linreg_slope.get(n - 2).copied().flatten() {
                        if let Some(last_intercept) =
                            self.linreg_intercept.get(n - 2).copied().flatten()
                        {
                            if let Some(last_cfo) = self.cmo.last_mut() {
                                if let Some(last) = self.bars.last() {
                                    let x = (n - 1) as f64;
                                    let forecast = last_slope * x + last_intercept;
                                    if last.close != 0.0 {
                                        *last_cfo =
                                            Some(100.0 * (last.close - forecast) / last.close);
                                    }
                                }
                            }
                        }
                    }
                }

                // CMO / QStick / Disparity / BOP / StdDev — GPU with CPU fallback
                let cmo_p = 9u32;
                if let Some(data) = gpu.compute_cmo_gpu(cmo_p) {
                    self.cmo = data
                        .iter()
                        .enumerate()
                        .map(|(i, &v)| {
                            if i < cmo_p as usize {
                                None
                            } else {
                                Some(v as f64)
                            }
                        })
                        .collect();
                } else {
                    self.cmo = compute_cmo(&self.bars, cmo_p as usize);
                }

                // O(1) forming-bar update for CMO
                if self.forming_bar_dirty && n > 1 && cmo_p as usize > 0 {
                    if let Some(last) = self.bars.last() {
                        let delta = last.close - self.bars[n - 2].close;
                        if delta > 0.0 {
                            self.cmo_sum_up += delta;
                        } else if delta < 0.0 {
                            self.cmo_sum_down += -delta;
                        }
                        let denom = self.cmo_sum_up + self.cmo_sum_down;
                        if let Some(last_cmo) = self.cmo.last_mut() {
                            *last_cmo = if denom > f64::EPSILON {
                                Some(100.0 * (self.cmo_sum_up - self.cmo_sum_down) / denom)
                            } else {
                                Some(0.0)
                            };
                        }
                    }
                }

                // O(1) forming-bar update for Linear Regression Slope (simple incremental)
                if self.forming_bar_dirty && n > 1 {
                    if let Some(last) = self.bars.last() {
                        let x = (n - 1) as f64; // current bar index
                        let y = last.close;
                        self.linreg_sum_x += x;
                        self.linreg_sum_y += y;
                        self.linreg_sum_xy += x * y;
                        self.linreg_sum_x2 += x * x;

                        let n_f = n as f64;
                        let denom =
                            n_f * self.linreg_sum_x2 - self.linreg_sum_x * self.linreg_sum_x;
                        if let Some(last_slope) = self.linreg_slope.last_mut() {
                            if denom > f64::EPSILON {
                                *last_slope = Some(
                                    (n_f * self.linreg_sum_xy
                                        - self.linreg_sum_x * self.linreg_sum_y)
                                        / denom,
                                );
                            } else {
                                *last_slope = Some(0.0);
                            }
                        }
                    }
                }

                let qstick_p = 14u32;
                if let Some(data) = gpu.compute_qstick_gpu(qstick_p) {
                    self.qstick = data
                        .iter()
                        .enumerate()
                        .map(|(i, &v)| {
                            if i + 1 < qstick_p as usize {
                                None
                            } else {
                                Some(v as f64)
                            }
                        })
                        .collect();
                } else {
                    self.qstick = compute_qstick(&self.bars, qstick_p as usize);
                }

                let disparity_p = 14u32;
                if let Some(data) = gpu.compute_disparity_gpu(disparity_p) {
                    self.disparity = data
                        .iter()
                        .enumerate()
                        .map(|(i, &v)| {
                            if i + 1 < disparity_p as usize {
                                None
                            } else {
                                Some(v as f64)
                            }
                        })
                        .collect();
                } else {
                    self.disparity = compute_disparity(&self.bars, disparity_p as usize);
                }

                // O(1) forming-bar update for Disparity (using existing SMA100)
                if self.forming_bar_dirty && n > 1 && disparity_p as usize == 100 {
                    if let Some(prev_sma) = self.sma100.get(n - 2).copied().flatten() {
                        if let Some(last_disp) = self.disparity.last_mut() {
                            if let Some(last) = self.bars.last() {
                                let new_ma = (prev_sma * 99.0 + last.close) / 100.0;
                                if new_ma != 0.0 {
                                    *last_disp = Some((last.close - new_ma) / new_ma);
                                }
                            }
                        }
                    }
                }

                let bop_p = 14u32;
                if let Some(data) = gpu.compute_bop_gpu(bop_p) {
                    self.bop = data
                        .iter()
                        .enumerate()
                        .map(|(i, &v)| {
                            if i + 1 < bop_p as usize {
                                None
                            } else {
                                Some(v as f64)
                            }
                        })
                        .collect();
                } else {
                    self.bop = compute_bop(&self.bars, bop_p as usize);
                }

                let stddev_p = 20u32;
                if let Some(data) = gpu.compute_stddev_gpu(stddev_p) {
                    self.stddev = data
                        .iter()
                        .enumerate()
                        .map(|(i, &v)| {
                            if i + 1 < stddev_p as usize {
                                None
                            } else {
                                Some(v as f64)
                            }
                        })
                        .collect();
                } else {
                    self.stddev = compute_stddev(&self.bars, stddev_p as usize);
                }

                let mfi_p = 14u32;
                if let Some(data) = gpu.compute_mfi_gpu(mfi_p) {
                    self.mfi = data
                        .iter()
                        .enumerate()
                        .map(|(i, &v)| {
                            if i < mfi_p as usize {
                                None
                            } else {
                                Some((v as f64).clamp(0.0, 100.0))
                            }
                        })
                        .collect();
                } else {
                    self.mfi = compute_mfi(&self.bars, mfi_p as usize);
                }

                let trix_p = 15u32;
                let trix_sig_p = 9u32;
                if let Some(data) = gpu.compute_trix_gpu(&self.upload_closes, trix_p, trix_sig_p) {
                    let trix_line_warmup = (3 * trix_p as usize).saturating_sub(2);
                    let trix_signal_warmup = 3 * trix_p as usize + trix_sig_p as usize - 3;
                    let mut line = Vec::with_capacity(n);
                    let mut signal = Vec::with_capacity(n);
                    let mut hist = Vec::with_capacity(n);
                    for i in 0..n {
                        let l = data.get(i * 3).copied().unwrap_or(0.0);
                        let s = data.get(i * 3 + 1).copied().unwrap_or(0.0);
                        let h = data.get(i * 3 + 2).copied().unwrap_or(0.0);
                        line.push(if i < trix_line_warmup {
                            None
                        } else {
                            Some(l as f64)
                        });
                        signal.push(if i < trix_signal_warmup {
                            None
                        } else {
                            Some(s as f64)
                        });
                        hist.push(if i < trix_signal_warmup {
                            None
                        } else {
                            Some(h as f64)
                        });
                    }
                    self.trix_line = line;
                    self.trix_signal = signal;
                    self.trix_hist = hist;
                } else {
                    let (line, signal, hist) =
                        compute_trix(&self.bars, trix_p as usize, trix_sig_p as usize);
                    self.trix_line = line;
                    self.trix_signal = signal;
                    self.trix_hist = hist;
                }

                let ppo_fast = 12u32;
                let ppo_slow = 26u32;
                let ppo_sig = 9u32;
                if let Some(data) =
                    gpu.compute_ppo_gpu(&self.upload_closes, ppo_fast, ppo_slow, ppo_sig)
                {
                    let ppo_line_warmup = ppo_slow as usize - 1;
                    let ppo_signal_warmup = ppo_slow as usize + ppo_sig as usize - 2;
                    let mut line = Vec::with_capacity(n);
                    let mut signal = Vec::with_capacity(n);
                    let mut hist = Vec::with_capacity(n);
                    for i in 0..n {
                        let l = data.get(i * 3).copied().unwrap_or(0.0);
                        let s = data.get(i * 3 + 1).copied().unwrap_or(0.0);
                        let h = data.get(i * 3 + 2).copied().unwrap_or(0.0);
                        line.push(if i < ppo_line_warmup {
                            None
                        } else {
                            Some(l as f64)
                        });
                        signal.push(if i < ppo_signal_warmup {
                            None
                        } else {
                            Some(s as f64)
                        });
                        hist.push(if i < ppo_signal_warmup {
                            None
                        } else {
                            Some(h as f64)
                        });
                    }
                    self.ppo_line = line;
                    self.ppo_signal = signal;
                    self.ppo_hist = hist;
                } else {
                    let (line, signal, hist) = compute_ppo(
                        &self.bars,
                        ppo_fast as usize,
                        ppo_slow as usize,
                        ppo_sig as usize,
                    );
                    self.ppo_line = line;
                    self.ppo_signal = signal;
                    self.ppo_hist = hist;
                }

                if let Some(data) = gpu.compute_ultosc_gpu() {
                    self.ultosc = data
                        .iter()
                        .enumerate()
                        .map(|(i, &v)| {
                            if i < 28 {
                                None
                            } else {
                                Some((v as f64).clamp(0.0, 100.0))
                            }
                        })
                        .collect();
                } else {
                    self.ultosc = compute_ultosc(&self.bars);
                }

                if let Some(data) = gpu.compute_stochrsi_gpu(&self.upload_closes, 14, 14, 3, 3) {
                    let stochrsi_k_warmup = 29usize;
                    let stochrsi_d_warmup = 31usize;
                    let mut k = Vec::with_capacity(n);
                    let mut d = Vec::with_capacity(n);
                    for i in 0..n {
                        let kv = data.get(i * 2).copied().unwrap_or(0.0);
                        let dv = data.get(i * 2 + 1).copied().unwrap_or(0.0);
                        k.push(if i < stochrsi_k_warmup {
                            None
                        } else {
                            Some((kv as f64).clamp(0.0, 100.0))
                        });
                        d.push(if i < stochrsi_d_warmup {
                            None
                        } else {
                            Some((dv as f64).clamp(0.0, 100.0))
                        });
                    }
                    self.stochrsi_k = k;
                    self.stochrsi_d = d;
                } else {
                    let (k, d) = compute_stochrsi(&self.bars, 14, 14, 3, 3);
                    self.stochrsi_k = k;
                    self.stochrsi_d = d;
                }

                // VaR Oscillator — GPU (sequential rolling 95% VaR, 0.0 is valid)
                let var_osc_p = 20u32;
                if let Some(data) = gpu.compute_var_oscillator_gpu(var_osc_p) {
                    self.var_oscillator = data
                        .iter()
                        .enumerate()
                        .map(|(i, &v)| {
                            if i < var_osc_p as usize || !v.is_finite() {
                                None
                            } else {
                                Some(v as f64)
                            }
                        })
                        .collect();
                } else {
                    self.var_oscillator = compute_var_oscillator(&self.bars, var_osc_p as usize);
                }

                // Parabolic SAR — GPU (sequential, from OHLC)
                if let Some(data) = gpu.compute_psar_gpu() {
                    self.psar = data
                        .iter()
                        .map(|&v| if v == 0.0 { None } else { Some(v as f64) })
                        .collect();
                } else {
                    self.psar = compute_parabolic_sar(&self.bars, 0.02, 0.2);
                }
                let (au, al) = compute_atr_projection(&self.bars, &self.atr);
                self.atr_proj_upper = au;
                self.atr_proj_lower = al;
                // ATR Projection — GPU (parallel: open ± ATR)
                {
                    let atrs: Vec<f32> = self.atr.iter().map(|v| v.unwrap_or(0.0) as f32).collect();
                    if let Some(data) = gpu.compute_atr_projection_gpu(&atrs) {
                        let n = self.bars.len();
                        let mut au = Vec::with_capacity(n);
                        let mut al = Vec::with_capacity(n);
                        for i in 0..n {
                            let u = data.get(i * 2).copied().unwrap_or(0.0);
                            let l = data.get(i * 2 + 1).copied().unwrap_or(0.0);
                            if u == 0.0 {
                                au.push(None);
                                al.push(None);
                            } else {
                                au.push(Some(u as f64));
                                al.push(Some(l as f64));
                            }
                        }
                        self.atr_proj_upper = au;
                        self.atr_proj_lower = al;
                    } else {
                        let (au, al) = compute_atr_projection(&self.bars, &self.atr);
                        self.atr_proj_upper = au;
                        self.atr_proj_lower = al;
                    }
                }

                // ATR Projection MTF levels (matching ATR_Projection.mqh)
                self.atr_proj_levels =
                    compute_atr_projection_levels(&self.bars, self.timeframe.minutes());

                // BetterVolume — GPU (full Emini-Watch algorithm with OHLCV)
                if let Some(data) = gpu.compute_better_volume_gpu_full(20) {
                    self.better_vol_type = data.iter().map(|&v| v as u8).collect();
                } else {
                    self.better_vol_type = compute_better_volume(&self.bars);
                }

                let (h1, h4, d1, w1, mn1) = compute_prev_candle_levels(&self.bars);
                self.prev_h1_high = h1.0;
                self.prev_h1_low = h1.1;
                self.prev_h4_high = h4.0;
                self.prev_h4_low = h4.1;
                self.prev_daily_high = d1.0;
                self.prev_daily_low = d1.1;
                self.prev_weekly_high = w1.0;
                self.prev_weekly_low = w1.1;
                self.prev_monthly_high = mn1.0;
                self.prev_monthly_low = mn1.1;
                let (cur_d1, cur_w1, cur_mn1) = compute_current_candle_levels(&self.bars);
                self.current_daily_high = cur_d1.0;
                self.current_daily_low = cur_d1.1;
                self.current_weekly_high = cur_w1.0;
                self.current_weekly_low = cur_w1.1;
                self.current_monthly_high = cur_mn1.0;
                self.current_monthly_low = cur_mn1.1;
                if let (Some(h), Some(l)) = (d1.0, d1.1) {
                    let prev_close = self
                        .bars
                        .iter()
                        .rev()
                        .find(|b| {
                            let day = b.ts_ms / 86_400_000;
                            let last_day = self
                                .bars
                                .last()
                                .map(|lb| lb.ts_ms / 86_400_000)
                                .unwrap_or(0);
                            day < last_day
                        })
                        .map(|b| b.close);
                    if let Some(c) = prev_close {
                        let p = (h + l + c) / 3.0;
                        self.pivot_p = Some(p);
                        self.pivot_r1 = Some(2.0 * p - l);
                        self.pivot_r2 = Some(p + (h - l));
                        self.pivot_s1 = Some(2.0 * p - h);
                        self.pivot_s2 = Some(p - (h - l));
                    }
                }

                // Fractals — GPU (parallel per-bar)
                if let Some(data) = gpu.compute_fractals_gpu() {
                    let n = self.bars.len();
                    self.fractal_up = vec![false; n];
                    self.fractal_down = vec![false; n];
                    for i in 0..n {
                        let up = data.get(i * 2).copied().unwrap_or(0.0);
                        let dn = data.get(i * 2 + 1).copied().unwrap_or(0.0);
                        if up != 0.0 {
                            self.fractal_up[i] = true;
                        }
                        if dn != 0.0 {
                            self.fractal_down[i] = true;
                        }
                    }
                } else {
                    self.fractal_up = compute_fractals_up(&self.bars);
                    self.fractal_down = compute_fractals_down(&self.bars);
                }

                self.harmonics =
                    detect_harmonic_patterns(&self.bars, &self.fractal_up, &self.fractal_down); // CPU (complex pattern matching)

                // Supply/Demand Zones — GPU fractal detection + CPU testing/merging
                // GPU Phase 1: detect fractals (parallel per-bar, 5-bar lookback)
                // CPU Phase 2: refine boundaries, test zones, merge, purge broken
                if let Some(data) = gpu.compute_sd_zones_gpu(5) {
                    let (sz, dz) = compute_supply_demand_zones_from_gpu(&data, &self.bars);
                    // GPU fallback: if GPU produces zero zones, try CPU
                    if sz.is_empty() && dz.is_empty() && self.bars.len() > 20 {
                        let (sz2, dz2) = compute_supply_demand_zones(&self.bars);
                        self.supply_zones = sz2;
                        self.demand_zones = dz2;
                        tracing::debug!(
                            "S/D: GPU produced 0 zones, CPU fallback: {} supply, {} demand",
                            self.supply_zones.len(),
                            self.demand_zones.len()
                        );
                    } else {
                        self.supply_zones = sz;
                        self.demand_zones = dz;
                    }
                } else {
                    let (sz, dz) = compute_supply_demand_zones(&self.bars);
                    self.supply_zones = sz;
                    self.demand_zones = dz;
                }
                self.compute_auto_fibonacci(); // CPU (fractal-based swing detection)
                // VWAP — GPU per-day segments with CPU deviation bands
                let gpu_vwap_ok = 'vwap_gpu: {
                    let n = self.bars.len();
                    if n == 0 {
                        break 'vwap_gpu false;
                    }

                    // Find day boundaries: indices where a new trading day starts
                    let mut day_starts: Vec<usize> = vec![0];
                    for i in 1..n {
                        let prev_day = self.bars[i - 1].ts_ms / 1000 / 86400;
                        let curr_day = self.bars[i].ts_ms / 1000 / 86400;
                        if curr_day != prev_day {
                            day_starts.push(i);
                        }
                    }

                    // Allocate output vectors
                    let mut vw = vec![None; n];
                    let mut vu1 = vec![None; n];
                    let mut vu2 = vec![None; n];
                    let mut vu3 = vec![None; n];
                    let mut vl1 = vec![None; n];
                    let mut vl2 = vec![None; n];
                    let mut vl3 = vec![None; n];

                    // Process each day segment on GPU
                    for seg_idx in 0..day_starts.len() {
                        let start = day_starts[seg_idx];
                        let end = if seg_idx + 1 < day_starts.len() {
                            day_starts[seg_idx + 1]
                        } else {
                            n
                        };
                        let seg_len = end - start;

                        // GPU: compute anchored VWAP for this day segment directly from resident
                        // OHLCV buffers without rebuilding per-segment scratch arrays.
                        let gpu_result = gpu.compute_anchored_vwap(start as u32, end as u32);
                        let gpu_vwap = match gpu_result {
                            Some(v) if v.len() >= seg_len => v,
                            _ => {
                                break 'vwap_gpu false;
                            }
                        };

                        // CPU: compute deviation bands from GPU VWAP values
                        // σ = sqrt( Σ(tp²·vol)/Σ(vol) - vwap² )
                        let mut cum_vol = 0.0_f64;
                        let mut cum_tp2_vol = 0.0_f64;
                        for j in 0..seg_len {
                            let b = &self.bars[start + j];
                            let tp = (b.high + b.low + b.close) / 3.0;
                            let vol = b.volume.max(1.0);
                            cum_vol += vol;
                            cum_tp2_vol += tp * tp * vol;

                            let vwap_val = gpu_vwap[j] as f64;
                            let variance = (cum_tp2_vol / cum_vol - vwap_val * vwap_val).max(0.0);
                            let sd = variance.sqrt();

                            let idx = start + j;
                            vw[idx] = Some(vwap_val);
                            vu1[idx] = Some(vwap_val + sd);
                            vu2[idx] = Some(vwap_val + 2.0 * sd);
                            vu3[idx] = Some(vwap_val + 3.0 * sd);
                            vl1[idx] = Some(vwap_val - sd);
                            vl2[idx] = Some(vwap_val - 2.0 * sd);
                            vl3[idx] = Some(vwap_val - 3.0 * sd);
                        }
                    }

                    self.vwap = vw;
                    self.vwap_upper1 = vu1;
                    self.vwap_upper2 = vu2;
                    self.vwap_upper3 = vu3;
                    self.vwap_lower1 = vl1;
                    self.vwap_lower2 = vl2;
                    self.vwap_lower3 = vl3;
                    true
                };
                if !gpu_vwap_ok {
                    // GPU VWAP failed — fall back to full CPU compute_vwap()
                    let (vw, vu1, vu2, vu3, vl1, vl2, vl3) = compute_vwap(&self.bars);
                    self.vwap = vw;
                    self.vwap_upper1 = vu1;
                    self.vwap_upper2 = vu2;
                    self.vwap_upper3 = vu3;
                    self.vwap_lower1 = vl1;
                    self.vwap_lower2 = vl2;
                    self.vwap_lower3 = vl3;
                }
                // Supertrend — GPU (sequential, ATR-based) with CPU fallback
                if let Some(data) = gpu.compute_supertrend_gpu(10) {
                    let n = self.bars.len();
                    let mut st = Vec::with_capacity(n);
                    let mut bull = Vec::with_capacity(n);
                    for i in 0..n {
                        let v = data.get(i * 2).copied().unwrap_or(0.0);
                        let d = data.get(i * 2 + 1).copied().unwrap_or(0.0);
                        if v == 0.0 {
                            st.push(None);
                        } else {
                            st.push(Some(v as f64));
                        }
                        bull.push(d > 0.0);
                    }
                    self.supertrend = st;
                    self.supertrend_bull = bull;
                } else {
                    let (st, st_bull) = compute_supertrend(&self.bars, &self.atr, 10, 3.0);
                    self.supertrend = st;
                    self.supertrend_bull = st_bull;
                }

                // Donchian Channel — GPU (parallel) with CPU fallback
                if let Some(data) = gpu.compute_donchian_gpu(20) {
                    let n = self.bars.len();
                    let mut du = Vec::with_capacity(n);
                    let mut dl = Vec::with_capacity(n);
                    for i in 0..n {
                        let u = data.get(i * 2).copied().unwrap_or(0.0);
                        let l = data.get(i * 2 + 1).copied().unwrap_or(0.0);
                        if u == 0.0 {
                            du.push(None);
                            dl.push(None);
                        } else {
                            du.push(Some(u as f64));
                            dl.push(Some(l as f64));
                        }
                    }
                    self.donchian_upper = du;
                    self.donchian_lower = dl;
                } else {
                    let (du, dl) = compute_donchian(&self.bars, 20);
                    self.donchian_upper = du;
                    self.donchian_lower = dl;
                }

                // Keltner Channel — GPU (sequential EMA+ATR) with CPU fallback
                if let Some(data) = gpu.compute_keltner_gpu(20) {
                    let n = self.bars.len();
                    let mut ku = Vec::with_capacity(n);
                    let mut km = Vec::with_capacity(n);
                    let mut kl = Vec::with_capacity(n);
                    for i in 0..n {
                        let u = data.get(i * 3).copied().unwrap_or(0.0);
                        let m = data.get(i * 3 + 1).copied().unwrap_or(0.0);
                        let l = data.get(i * 3 + 2).copied().unwrap_or(0.0);
                        if m == 0.0 {
                            ku.push(None);
                            km.push(None);
                            kl.push(None);
                        } else {
                            ku.push(Some(u as f64));
                            km.push(Some(m as f64));
                            kl.push(Some(l as f64));
                        }
                    }
                    self.keltner_upper = ku;
                    self.keltner_mid = km;
                    self.keltner_lower = kl;
                } else {
                    let (km, ku, kl) = compute_keltner(&self.bars, 20, 10, 1.5);
                    self.keltner_mid = km;
                    self.keltner_upper = ku;
                    self.keltner_lower = kl;
                }

                // Regression Channel — GPU (parallel least squares) with CPU fallback
                if let Some(data) = gpu.compute_regression_gpu(20) {
                    let n = self.bars.len();
                    let mut rm = Vec::with_capacity(n);
                    let mut ru = Vec::with_capacity(n);
                    let mut rl = Vec::with_capacity(n);
                    for i in 0..n {
                        let m = data.get(i * 3).copied().unwrap_or(0.0);
                        let u = data.get(i * 3 + 1).copied().unwrap_or(0.0);
                        let l = data.get(i * 3 + 2).copied().unwrap_or(0.0);
                        if m == 0.0 {
                            rm.push(None);
                            ru.push(None);
                            rl.push(None);
                        } else {
                            rm.push(Some(m as f64));
                            ru.push(Some(u as f64));
                            rl.push(Some(l as f64));
                        }
                    }
                    self.regression_mid = rm;
                    self.regression_upper = ru;
                    self.regression_lower = rl;
                } else {
                    let (rm, ru, rl) = compute_regression_channel(&self.bars, 20);
                    self.regression_mid = rm;
                    self.regression_upper = ru;
                    self.regression_lower = rl;
                }

                // Squeeze Momentum — GPU (sequential BB+KC) with CPU fallback
                if let Some(data) = gpu.compute_squeeze_gpu(20) {
                    let n = self.bars.len();
                    let mut sm = Vec::with_capacity(n);
                    let mut sq = Vec::with_capacity(n);
                    for i in 0..n {
                        let m = data.get(i * 2).copied().unwrap_or(0.0);
                        let s = data.get(i * 2 + 1).copied().unwrap_or(0.0);
                        sm.push(Some(m as f64));
                        sq.push(s > 0.5);
                    }
                    self.squeeze_mom = sm;
                    self.squeeze_on = sq;
                } else {
                    let (sm, sq) = compute_squeeze_momentum(
                        &self.bb_upper,
                        &self.bb_lower,
                        &self.keltner_upper,
                        &self.keltner_lower,
                        &self.bars,
                        20,
                    );
                    self.squeeze_mom = sm;
                    self.squeeze_on = sq;
                }
                // Pre-compute 20-bar rolling average volume for heatmap candle coloring
                {
                    let n = self.bars.len();
                    let mut avg = vec![0.0_f64; n];
                    let period = 20usize;
                    let mut sum = 0.0;
                    for i in 0..n {
                        sum += self.bars[i].volume;
                        if i >= period {
                            sum -= self.bars[i - period].volume;
                        }
                        avg[i] = if i >= period - 1 {
                            sum / period as f64
                        } else {
                            sum / (i + 1) as f64
                        };
                    }
                    self.vol_avg_20 = avg;
                }

                // Ehlers Super Smoother — GPU
                if let Some(data) = gpu.compute_ehlers_ss_gpu(10) {
                    self.ehlers_ss = data
                        .iter()
                        .map(|&v| if v == 0.0 { None } else { Some(v as f64) })
                        .collect();
                } else {
                    self.ehlers_ss = ehlers_super_smoother(&self.bars, 10);
                }

                // Ehlers Decycler — GPU
                if let Some(data) = gpu.compute_ehlers_dec_gpu(20) {
                    self.ehlers_decycler = data
                        .iter()
                        .map(|&v| if v == 0.0 { None } else { Some(v as f64) })
                        .collect();
                } else {
                    self.ehlers_decycler = ehlers_decycler(&self.bars, 20);
                }

                // Ehlers ITL — GPU
                if let Some(data) = gpu.compute_ehlers_itl_gpu() {
                    self.ehlers_itl = data
                        .iter()
                        .map(|&v| if v == 0.0 { None } else { Some(v as f64) })
                        .collect();
                } else {
                    self.ehlers_itl = ehlers_instantaneous_trendline(&self.bars);
                }

                // Ehlers MAMA/FAMA — GPU (2 outputs)
                if let Some(data) = gpu.compute_ehlers_mama_gpu() {
                    let n = self.bars.len();
                    let mut mama = Vec::with_capacity(n);
                    let mut fama = Vec::with_capacity(n);
                    for i in 0..n {
                        let m = data.get(i * 2).copied().unwrap_or(0.0);
                        let f = data.get(i * 2 + 1).copied().unwrap_or(0.0);
                        if i < 6 || (m == 0.0 && f == 0.0) {
                            mama.push(None);
                            fama.push(None);
                        } else {
                            mama.push(Some(m as f64));
                            fama.push(Some(f as f64));
                        }
                    }
                    self.ehlers_mama = mama;
                    self.ehlers_fama = fama;
                } else {
                    let (m, f) = ehlers_mama_fama(&self.bars, 0.5, 0.05);
                    self.ehlers_mama = m;
                    self.ehlers_fama = f;
                }

                // Ehlers EBSW — GPU (sub-pane oscillator, CPU starts at i=1)
                if let Some(data) = gpu.compute_ehlers_ebsw_gpu(40) {
                    self.ehlers_ebsw = data
                        .iter()
                        .enumerate()
                        .map(|(i, &v)| if i < 2 { None } else { Some(v as f64) })
                        .collect();
                } else {
                    self.ehlers_ebsw = ehlers_even_better_sinewave(&self.bars, 40);
                }

                // Ehlers Cyber Cycle — GPU (sub-pane oscillator, CPU starts at i=4)
                if let Some(data) = gpu.compute_ehlers_cyber_gpu() {
                    self.ehlers_cyber = data
                        .iter()
                        .enumerate()
                        .map(|(i, &v)| if i < 4 { None } else { Some(v as f64) })
                        .collect();
                } else {
                    self.ehlers_cyber = ehlers_cyber_cycle(&self.bars);
                }

                // Ehlers CG Oscillator — GPU (parallel, CPU starts at period-1=9, 0.0 is valid)
                if let Some(data) = gpu.compute_ehlers_cg_gpu(10) {
                    self.ehlers_cg = data
                        .iter()
                        .enumerate()
                        .map(|(i, &v)| if i < 9 { None } else { Some(v as f64) })
                        .collect();
                } else {
                    self.ehlers_cg = ehlers_cg_oscillator(&self.bars, 10);
                }

                // Ehlers Roofing Filter — GPU (sub-pane oscillator, CPU starts at i=2)
                if let Some(data) = gpu.compute_ehlers_roof_gpu(10, 48) {
                    self.ehlers_roof = data
                        .iter()
                        .enumerate()
                        .map(|(i, &v)| if i < 2 { None } else { Some(v as f64) })
                        .collect();
                } else {
                    self.ehlers_roof = ehlers_roofing_filter(&self.bars, 10, 48);
                }
                return;
            }
        }

        // ── CPU fallback path (no GPU available) ──
        let sma_slow = self.sma_slow_period as usize;
        let sma_fast = self.sma_fast_period as usize;
        let ema_p = self.ema_period as usize;
        let bb_p = self.bb_period as usize;
        let rsi_p = self.rsi_period as usize;
        let fisher_p = self.fisher_period as usize;
        let atr_p = self.atr_period as usize;
        let stoch_p = self.stoch_period as usize;
        let adx_p = self.adx_period as usize;
        let mom_p = self.momentum_period as usize;
        self.sma200 = compute_sma(&self.bars, sma_slow);
        self.sma100 = compute_sma(&self.bars, sma_fast);
        self.kama = compute_kama(&self.bars, 10, 2, 30);
        self.ema21 = compute_ema(&self.bars, ema_p);
        let (mid, upper, lower) = compute_bollinger(&self.bars, bb_p, 2.0);
        self.bb_mid = mid;
        self.bb_upper = upper;
        self.bb_lower = lower;
        self.rsi = compute_rsi(&self.bars, rsi_p);
        let (f, fs) = compute_fisher(&self.bars, fisher_p);
        self.fisher = f;
        self.fisher_signal = fs;
        self.atr = compute_atr(&self.bars, atr_p);
        let (ml, ms, mh) = compute_macd(
            &self.bars,
            self.macd_fast as usize,
            self.macd_slow as usize,
            self.macd_signal_p as usize,
        );
        self.macd_line = ml;
        self.macd_signal = ms;
        self.macd_hist = mh;
        let (sk, sd) = compute_stochastic(&self.bars, stoch_p, 3, 3);
        self.stoch_k = sk;
        self.stoch_d = sd;
        let (adx, dip, dim) = compute_adx(&self.bars, adx_p);
        self.adx = adx;
        self.di_plus = dip;
        self.di_minus = dim;
        let (tk, kj, sa, sb) = compute_ichimoku(&self.bars, 9, 26, 52);
        self.ichi_tenkan = tk;
        self.ichi_kijun = kj;
        self.ichi_span_a = sa;
        self.ichi_span_b = sb;
        self.wma = compute_wma(&self.bars, 20);
        self.hma = compute_hma(&self.bars, 20);
        self.cci = compute_cci(&self.bars, 20);
        self.williams_r = compute_williams_r(&self.bars, 14);
        self.obv = compute_obv(&self.bars);
        self.momentum = compute_momentum(&self.bars, mom_p);
        self.cmo = compute_cmo(&self.bars, 9);
        self.qstick = compute_qstick(&self.bars, 14);
        self.disparity = compute_disparity(&self.bars, 14);
        self.bop = compute_bop(&self.bars, 14);
        self.stddev = compute_stddev(&self.bars, 20);
        self.mfi = compute_mfi(&self.bars, 14);
        let (trix_line, trix_signal, trix_hist) = compute_trix(&self.bars, 15, 9);
        self.trix_line = trix_line;
        self.trix_signal = trix_signal;
        self.trix_hist = trix_hist;
        let (ppo_line, ppo_signal, ppo_hist) = compute_ppo(&self.bars, 12, 26, 9);
        self.ppo_line = ppo_line;
        self.ppo_signal = ppo_signal;
        self.ppo_hist = ppo_hist;
        self.ultosc = compute_ultosc(&self.bars);
        let (stochrsi_k, stochrsi_d) = compute_stochrsi(&self.bars, 14, 14, 3, 3);
        self.stochrsi_k = stochrsi_k;
        self.stochrsi_d = stochrsi_d;
        self.var_oscillator = compute_var_oscillator(&self.bars, 20);
        self.psar = compute_parabolic_sar(&self.bars, 0.02, 0.2);
        let (au, al) = compute_atr_projection(&self.bars, &self.atr);
        self.atr_proj_upper = au;
        self.atr_proj_lower = al;
        self.atr_proj_levels = compute_atr_projection_levels(&self.bars, self.timeframe.minutes());
        self.better_vol_type = compute_better_volume(&self.bars);
        // Previous candle levels — find the second-to-last daily/weekly bar boundaries
        let (h1, h4, d1, w1, mn1) = compute_prev_candle_levels(&self.bars);
        self.prev_h1_high = h1.0;
        self.prev_h1_low = h1.1;
        self.prev_h4_high = h4.0;
        self.prev_h4_low = h4.1;
        self.prev_daily_high = d1.0;
        self.prev_daily_low = d1.1;
        self.prev_weekly_high = w1.0;
        self.prev_weekly_low = w1.1;
        self.prev_monthly_high = mn1.0;
        self.prev_monthly_low = mn1.1;
        let (cur_d1, cur_w1, cur_mn1) = compute_current_candle_levels(&self.bars);
        self.current_daily_high = cur_d1.0;
        self.current_daily_low = cur_d1.1;
        self.current_weekly_high = cur_w1.0;
        self.current_weekly_low = cur_w1.1;
        self.current_monthly_high = cur_mn1.0;
        self.current_monthly_low = cur_mn1.1;
        // Pivot points from previous day
        if let (Some(h), Some(l)) = (d1.0, d1.1) {
            // Hoist last_day out of the find closure — was recomputed on every
            // bar iteration during the reverse scan.
            let last_day = self
                .bars
                .last()
                .map(|lb| lb.ts_ms / 86_400_000)
                .unwrap_or(0);
            let prev_close = self
                .bars
                .iter()
                .rev()
                .find(|b| b.ts_ms / 86_400_000 < last_day)
                .map(|b| b.close);
            if let Some(c) = prev_close {
                let p = (h + l + c) / 3.0;
                self.pivot_p = Some(p);
                self.pivot_r1 = Some(2.0 * p - l);
                self.pivot_r2 = Some(p + (h - l));
                self.pivot_s1 = Some(2.0 * p - h);
                self.pivot_s2 = Some(p - (h - l));
            }
        }
        // Fractals
        self.fractal_up = compute_fractals_up(&self.bars);
        self.fractal_down = compute_fractals_down(&self.bars);
        self.harmonics = detect_harmonic_patterns(&self.bars, &self.fractal_up, &self.fractal_down);
        let (sz, dz) = compute_supply_demand_zones(&self.bars);
        self.supply_zones = sz;
        self.demand_zones = dz;
        // Auto Fibonacci (fractal-based swing detection, matching AutoFibonacci.mqh)
        self.compute_auto_fibonacci();
        // VWAP (daily anchored)
        let (vw, vu1, vu2, vu3, vl1, vl2, vl3) = compute_vwap(&self.bars);
        self.vwap = vw;
        self.vwap_upper1 = vu1;
        self.vwap_upper2 = vu2;
        self.vwap_upper3 = vu3;
        self.vwap_lower1 = vl1;
        self.vwap_lower2 = vl2;
        self.vwap_lower3 = vl3;
        // Supertrend, Donchian, Keltner
        let (st, st_bull) = compute_supertrend(&self.bars, &self.atr, 10, 3.0);
        self.supertrend = st;
        self.supertrend_bull = st_bull;
        let (du, dl) = compute_donchian(&self.bars, 20);
        self.donchian_upper = du;
        self.donchian_lower = dl;
        let (km, ku, kl) = compute_keltner(&self.bars, 20, 10, 1.5);
        self.keltner_mid = km;
        self.keltner_upper = ku;
        self.keltner_lower = kl;
        let (rm, ru, rl) = compute_regression_channel(&self.bars, 20);
        self.regression_mid = rm;
        self.regression_upper = ru;
        self.regression_lower = rl;
        let (sm, sq) = compute_squeeze_momentum(
            &self.bb_upper,
            &self.bb_lower,
            &self.keltner_upper,
            &self.keltner_lower,
            &self.bars,
            20,
        );
        self.squeeze_mom = sm;
        self.squeeze_on = sq;
        // Pre-compute 20-bar rolling average volume for heatmap candle coloring
        {
            let n = self.bars.len();
            let mut avg = vec![0.0_f64; n];
            let period = 20usize;
            let mut sum = 0.0;
            for i in 0..n {
                sum += self.bars[i].volume;
                if i >= period {
                    sum -= self.bars[i - period].volume;
                }
                avg[i] = if i >= period - 1 {
                    sum / period as f64
                } else {
                    sum / (i + 1) as f64
                };
            }
            self.vol_avg_20 = avg;
        }
        // Ehlers indicators
        self.ehlers_ss = ehlers_super_smoother(&self.bars, 10);
        self.ehlers_decycler = ehlers_decycler(&self.bars, 20);
        self.ehlers_itl = ehlers_instantaneous_trendline(&self.bars);
        let (mama, fama) = ehlers_mama_fama(&self.bars, 0.5, 0.05);
        self.ehlers_mama = mama;
        self.ehlers_fama = fama;
        self.ehlers_ebsw = ehlers_even_better_sinewave(&self.bars, 40);
        self.ehlers_cyber = ehlers_cyber_cycle(&self.bars);
        self.ehlers_cg = ehlers_cg_oscillator(&self.bars, 10);
        self.ehlers_roof = ehlers_roofing_filter(&self.bars, 10, 48);
    }
}
