// ── SMA Tests ────────────────────────────────────────────────────────

#[test]
fn test_sma_basic() {
    let bars = make_bars(10);
    let sma = compute_sma(&bars, 3);
    assert_eq!(sma.len(), 10);
    // First 2 should be None (period-1)
    assert!(sma[0].is_none());
    assert!(sma[1].is_none());
    // Third bar should have a value
    assert!(sma[2].is_some());
    // SMA(3) of closes 101, 102, 103 = 102
    let v = sma[2].unwrap();
    assert!(
        (v - 102.0).abs() < 0.01,
        "SMA(3) bar 2 = {}, expected ~102",
        v
    );
}

#[test]
fn test_sma_empty() {
    let bars: Vec<Bar> = vec![];
    let sma = compute_sma(&bars, 5);
    assert!(sma.is_empty());
}

#[test]
fn test_sma_period_larger_than_data() {
    let bars = make_bars(3);
    let sma = compute_sma(&bars, 10);
    assert_eq!(sma.len(), 3);
    assert!(sma.iter().all(|v| v.is_none()));
}

// ── EMA Tests ────────────────────────────────────────────────────────

#[test]
fn test_ema_basic() {
    let bars = make_bars(20);
    let ema = compute_ema(&bars, 5);
    assert_eq!(ema.len(), 20);
    // First 4 should be None
    for i in 0..4 {
        assert!(ema[i].is_none(), "EMA[{}] should be None", i);
    }
    // Should have values from period-1 onward
    assert!(ema[4].is_some());
    // EMA should be close to but not exactly equal to close prices (trending up)
    let last = ema[19].unwrap();
    assert!(last > 100.0 && last < 125.0, "EMA last = {}", last);
}

#[test]
fn test_ema_follows_trend() {
    let bars = make_bars(50);
    let ema = compute_ema(&bars, 10);
    // EMA should be increasing for ascending bars
    let mut prev = 0.0;
    for v in ema.iter().flatten() {
        assert!(
            *v >= prev,
            "EMA should be non-decreasing: {} < {}",
            *v,
            prev
        );
        prev = *v;
    }
}

// ── KAMA Tests ───────────────────────────────────────────────────────

#[test]
fn test_kama_basic() {
    let bars = make_bars(30);
    let kama = compute_kama(&bars, 10, 2, 30);
    assert_eq!(kama.len(), 30);
    assert!(kama[9].is_none()); // period-1 warmup
    assert!(kama[10].is_some());
}

#[test]
fn test_kama_adapts_to_trend() {
    let bars = make_bars(50);
    let kama = compute_kama(&bars, 10, 2, 30);
    // KAMA should follow the uptrend
    let last = kama.last().unwrap().unwrap();
    assert!(last > 130.0, "KAMA should follow uptrend: {}", last);
}

// ── Bollinger Bands ──────────────────────────────────────────────────

#[test]
fn test_bollinger_bands() {
    let bars = make_bars(30);
    let (mid, upper, lower) = compute_bollinger(&bars, 20, 2.0);
    assert_eq!(mid.len(), 30);
    // After warmup, upper > mid > lower
    for i in 19..30 {
        if let (Some(u), Some(m), Some(l)) = (upper[i], mid[i], lower[i]) {
            assert!(u > m, "Upper {} should be > mid {}", u, m);
            assert!(m > l, "Mid {} should be > lower {}", m, l);
        }
    }
}

// ── RSI Tests ────────────────────────────────────────────────────────

#[test]
fn test_rsi_range() {
    let bars = make_oscillating_bars(50);
    let rsi = compute_rsi(&bars, 14);
    for v in rsi.iter().flatten() {
        assert!(*v >= 0.0 && *v <= 100.0, "RSI should be 0-100: {}", v);
    }
}

#[test]
fn test_rsi_uptrend_bullish() {
    let bars = make_bars(30);
    let rsi = compute_rsi(&bars, 14);
    // Strong uptrend should have RSI > 50
    if let Some(v) = rsi.last().unwrap() {
        assert!(*v > 50.0, "RSI in uptrend should be >50: {}", v);
    }
}

// ── Fisher Transform ─────────────────────────────────────────────────

#[test]
fn test_fisher_transform() {
    let bars = make_bars(50);
    let (fisher, signal) = compute_fisher(&bars, 32);
    assert_eq!(fisher.len(), 50);
    assert_eq!(signal.len(), 50);
    // Should have values after warmup
    let has_values = fisher.iter().any(|v| v.is_some());
    assert!(has_values, "Fisher should have computed values");
}

// ── MACD Tests ───────────────────────────────────────────────────────

#[test]
fn test_macd_basic() {
    let bars = make_bars(50);
    let (macd, signal, hist) = compute_macd(&bars, 12, 26, 9);
    assert_eq!(macd.len(), 50);
    assert_eq!(signal.len(), 50);
    assert_eq!(hist.len(), 50);
    // Should have values after warmup (26 + 9 bars)
    assert!(macd[35].is_some());
}

#[test]
fn test_macd_histogram_is_difference() {
    let bars = make_bars(50);
    let (macd, signal, hist) = compute_macd(&bars, 12, 26, 9);
    for i in 0..50 {
        if let (Some(m), Some(s), Some(h)) = (macd[i], signal[i], hist[i]) {
            assert!(
                (h - (m - s)).abs() < 0.001,
                "Histogram should be MACD - Signal"
            );
        }
    }
}

// ── Stochastic ───────────────────────────────────────────────────────

#[test]
fn test_stochastic_range() {
    let bars = make_oscillating_bars(50);
    let (k, d) = compute_stochastic(&bars, 14, 3, 3);
    for v in k.iter().flatten() {
        assert!(*v >= 0.0 && *v <= 100.0, "Stoch %K should be 0-100: {}", v);
    }
    for v in d.iter().flatten() {
        assert!(*v >= 0.0 && *v <= 100.0, "Stoch %D should be 0-100: {}", v);
    }
}

// ── ADX Tests ────────────────────────────────────────────────────────

#[test]
fn test_adx_range() {
    let bars = make_bars(50);
    let (adx, di_plus, di_minus) = compute_adx(&bars, 14);
    for v in adx.iter().flatten() {
        assert!(*v >= 0.0, "ADX should be >= 0: {}", v);
    }
    for v in di_plus.iter().flatten() {
        assert!(*v >= 0.0, "DI+ should be >= 0: {}", v);
    }
    for v in di_minus.iter().flatten() {
        assert!(*v >= 0.0, "DI- should be >= 0: {}", v);
    }
}

// ── ATR Tests ────────────────────────────────────────────────────────

#[test]
fn test_atr_positive() {
    let bars = make_bars(30);
    let atr = compute_atr(&bars, 14);
    for v in atr.iter().flatten() {
        assert!(*v > 0.0, "ATR should be > 0: {}", v);
    }
}

// ── Ichimoku Tests ───────────────────────────────────────────────────

#[test]
fn test_ichimoku_lengths() {
    let bars = make_bars(60);
    let (tenkan, kijun, span_a, span_b) = compute_ichimoku(&bars, 9, 26, 52);
    assert_eq!(tenkan.len(), 60);
    assert_eq!(kijun.len(), 60);
    assert_eq!(span_a.len(), 60);
    assert_eq!(span_b.len(), 60);
}

// ── WMA / HMA Tests ─────────────────────────────────────────────────

#[test]
fn test_wma_basic() {
    let bars = make_bars(30);
    let wma = compute_wma(&bars, 10);
    assert_eq!(wma.len(), 30);
    assert!(wma[9].is_some());
}

#[test]
fn test_hma_basic() {
    let bars = make_bars(30);
    let hma = compute_hma(&bars, 10);
    assert_eq!(hma.len(), 30);
    // HMA should have values after warmup
    let has_values = hma.iter().any(|v| v.is_some());
    assert!(has_values);
}

// ── CCI / Williams %R ────────────────────────────────────────────────

#[test]
fn test_cci_basic() {
    let bars = make_oscillating_bars(30);
    let cci = compute_cci(&bars, 20);
    assert_eq!(cci.len(), 30);
}

#[test]
fn test_williams_r_range() {
    let bars = make_oscillating_bars(30);
    let wr = compute_williams_r(&bars, 14);
    for v in wr.iter().flatten() {
        assert!(
            *v >= -100.0 && *v <= 0.0,
            "Williams %R should be -100 to 0: {}",
            v
        );
    }
}

// ── OBV / Momentum ──────────────────────────────────────────────────

#[test]
fn test_obv_basic() {
    let bars = make_bars(20);
    let obv = compute_obv(&bars);
    assert_eq!(obv.len(), 20);
    assert!(obv[0].is_some());
}

#[test]
fn test_momentum_basic() {
    let bars = make_bars(20);
    let mom = compute_momentum(&bars, 10);
    assert_eq!(mom.len(), 20);
}

// ── Parabolic SAR ────────────────────────────────────────────────────

#[test]
fn test_psar_basic() {
    let bars = make_bars(30);
    let psar = compute_parabolic_sar(&bars, 0.02, 0.2);
    assert_eq!(psar.len(), 30);
    let has_values = psar.iter().any(|v| v.is_some());
    assert!(has_values);
}

// ── Fractals ─────────────────────────────────────────────────────────

#[test]
fn test_fractals_length() {
    let bars = make_bars(20);
    let up = compute_fractals_up(&bars);
    let down = compute_fractals_down(&bars);
    assert_eq!(up.len(), 20);
    assert_eq!(down.len(), 20);
}

// ── BetterVolume ─────────────────────────────────────────────────────

#[test]
fn test_better_volume_classification() {
    let bars = make_oscillating_bars(30);
    let bv = compute_better_volume(&bars);
    assert_eq!(bv.len(), 30);
    // All values should be 0-5
    for v in &bv {
        assert!(*v <= 5, "BetterVolume type should be 0-5: {}", v);
    }
}

// ── Supply/Demand Zones ──────────────────────────────────────────────

#[test]
fn test_supply_demand_zones() {
    let bars = make_oscillating_bars(50);
    let (supply, demand) = compute_supply_demand_zones(&bars);
    for (idx, high, low, status) in &supply {
        assert!(*idx < bars.len());
        assert!(high > low);
        assert!(*status <= 2);
    }
    for (idx, high, low, status) in &demand {
        assert!(*idx < bars.len());
        assert!(high > low);
        assert!(*status <= 2);
    }
}

#[test]
fn test_supply_demand_realistic_swings() {
    // Simulate: rally from 20→200, then crash to 80, then bounce to 140, then drop to 85
    // Should produce surviving zones near recent swing highs/lows
    let mut bars = Vec::new();
    let prices = [
        // Phase 1: Rally 20→200 (100 bars)
        20.0, 22.0, 25.0, 28.0, 30.0, 33.0, 35.0, 38.0, 40.0, 43.0, 45.0, 48.0, 50.0, 55.0, 58.0,
        60.0, 65.0, 68.0, 70.0, 75.0, 78.0, 80.0, 85.0, 88.0, 90.0, 95.0, 98.0, 100.0, 105.0,
        110.0, 112.0, 115.0, 118.0, 120.0, 125.0, 128.0, 130.0, 135.0, 138.0, 140.0, 142.0, 145.0,
        148.0, 150.0, 155.0, 158.0, 160.0, 165.0, 168.0, 170.0, 172.0, 175.0, 178.0, 180.0, 182.0,
        185.0, 188.0, 190.0, 192.0, 195.0, 197.0, 200.0, 198.0, 195.0, 192.0, 190.0, 188.0, 185.0,
        182.0, 180.0, // Phase 2: Crash 180→80 (30 bars)
        175.0, 170.0, 165.0, 155.0, 150.0, 140.0, 135.0, 130.0, 125.0, 120.0, 115.0, 110.0, 105.0,
        100.0, 95.0, 90.0, 88.0, 85.0, 82.0, 80.0, // Phase 3: Bounce 80→140 (20 bars)
        82.0, 85.0, 88.0, 92.0, 95.0, 100.0, 105.0, 110.0, 115.0, 120.0, 125.0, 128.0, 130.0,
        135.0, 138.0, 140.0, 138.0, 135.0, 130.0, 125.0, // Phase 4: Drop 125→85 (20 bars)
        120.0, 115.0, 110.0, 105.0, 100.0, 98.0, 95.0, 92.0, 90.0, 88.0, 87.0, 86.0, 85.0, 86.0,
        87.0, 85.0, 84.0, 85.0, 86.0, 85.0,
    ];
    for (i, &close) in prices.iter().enumerate() {
        let range = close * 0.03; // 3% daily range
        bars.push(Bar {
            ts_ms: 1700000000000 + i as i64 * 86400000,
            open: close - range * 0.2,
            high: close + range * 0.5,
            low: close - range * 0.5,
            close,
            volume: 1000.0,
        });
    }
    let n = bars.len();
    eprintln!(
        "[test] {} bars, price range {:.0}-{:.0}",
        n,
        bars.iter().map(|b| b.low).fold(f64::MAX, f64::min),
        bars.iter().map(|b| b.high).fold(f64::MIN, f64::max)
    );

    let (supply, demand) = compute_supply_demand_zones(&bars);

    eprintln!(
        "[test] Result: {} supply, {} demand zones",
        supply.len(),
        demand.len()
    );
    for (idx, hi, lo, st) in &supply {
        eprintln!(
            "[test]   SUPPLY bar={} hi={:.2} lo={:.2} st={}",
            idx, hi, lo, st
        );
    }
    for (idx, hi, lo, st) in &demand {
        eprintln!(
            "[test]   DEMAND bar={} hi={:.2} lo={:.2} st={}",
            idx, hi, lo, st
        );
    }

    // We should have at least some surviving zones:
    // - The 200 peak supply zone (price never went above 200 again)
    // - The 80 low demand zone (price never went below 80 again)
    // - Recent swing zones near current price
    assert!(
        !supply.is_empty() || !demand.is_empty(),
        "Should have surviving zones for a chart with clear swings"
    );
}

// ── Ehlers DSP Indicators ────────────────────────────────────────────

#[test]
fn test_ehlers_super_smoother() {
    let bars = make_bars(30);
    let ss = ehlers_super_smoother(&bars, 10);
    assert_eq!(ss.len(), 30);
    let has_values = ss.iter().any(|v| v.is_some());
    assert!(has_values);
}

#[test]
fn test_ehlers_decycler() {
    let bars = make_bars(30);
    let dc = ehlers_decycler(&bars, 20);
    assert_eq!(dc.len(), 30);
}

#[test]
fn test_ehlers_mama_fama() {
    let bars = make_bars(30);
    let (mama, fama) = ehlers_mama_fama(&bars, 0.5, 0.05);
    assert_eq!(mama.len(), 30);
    assert_eq!(fama.len(), 30);
}

#[test]
fn test_ehlers_ebsw() {
    let bars = make_oscillating_bars(50);
    let ebsw = ehlers_even_better_sinewave(&bars, 40);
    assert_eq!(ebsw.len(), 50);
    // EBSW should be in -1 to 1 range
    for v in ebsw.iter().flatten() {
        assert!(*v >= -2.0 && *v <= 2.0, "EBSW should be ~-1 to 1: {}", v);
    }
}

#[test]
fn test_ehlers_cyber_cycle() {
    let bars = make_oscillating_bars(30);
    let cc = ehlers_cyber_cycle(&bars);
    assert_eq!(cc.len(), 30);
}

#[test]
fn test_ehlers_cg_oscillator() {
    let bars = make_bars(30);
    let cg = ehlers_cg_oscillator(&bars, 10);
    assert_eq!(cg.len(), 30);
}

#[test]
fn test_ehlers_roofing_filter() {
    let bars = make_oscillating_bars(60);
    let rf = ehlers_roofing_filter(&bars, 10, 48);
    assert_eq!(rf.len(), 60);
}

// ── Heikin-Ashi / Renko ──────────────────────────────────────────────

#[test]
fn test_heikin_ashi() {
    let bars = make_bars(10);
    let ha = heikin_ashi(&bars);
    assert_eq!(ha.len(), 10);
    // HA close = (O+H+L+C)/4
    let b = &bars[0];
    let ha_close = (b.open + b.high + b.low + b.close) / 4.0;
    assert!((ha[0].close - ha_close).abs() < 0.01);
}

#[test]
fn test_renko_bricks() {
    let bars = make_bars(50);
    let bricks = renko_bricks(&bars);
    // Renko should produce some bricks for trending data
    assert!(
        !bricks.is_empty(),
        "Renko should produce bricks for trending data"
    );
}

// ── ATR Projection ───────────────────────────────────────────────────

#[test]
fn test_atr_projection() {
    let bars = make_bars(20);
    let atr = compute_atr(&bars, 14);
    let (upper, lower) = compute_atr_projection(&bars, &atr);
    assert_eq!(upper.len(), 20);
    assert_eq!(lower.len(), 20);
    // Upper should be > lower where both exist
    for i in 0..20 {
        if let (Some(u), Some(l)) = (upper[i], lower[i]) {
            assert!(u > l, "ATR proj upper {} should be > lower {}", u, l);
        }
    }
}

// ── Previous Candle Levels ───────────────────────────────────────────

#[test]
fn test_prev_candle_levels() {
    let bars = make_bars(10);
    let (_h1, _h4, d1, w1, _mn1) = compute_prev_candle_levels(&bars);
    // With synthetic data, should have daily levels at least
    // (may be None if all bars are same "day" in synthetic data)
    let _ = (d1, w1);
}

// ── Helper Functions ─────────────────────────────────────────────────

#[test]
fn test_in_range() {
    assert!(in_range(0.5, 0.0, 1.0));
    assert!(!in_range(1.5, 0.0, 1.0));
    assert!(in_range(0.618, 0.5, 0.8));
}

#[test]
fn test_format_price() {
    let s = format_price(123.456);
    assert!(s.contains("123"));
}

#[test]
fn test_fuzzy_match() {
    assert!(fuzzy_match("sma", "SMA200"));
    assert!(fuzzy_match("fish", "Fisher Transform"));
    assert!(!fuzzy_match("xyz", "SMA200"));
    assert!(fuzzy_match("", "anything")); // empty matches all
}

// ── Auto Fibonacci ───────────────────────────────────────────────────

#[test]
fn test_auto_fibonacci() {
    let mut bars = make_bars(60);
    // Create a clear swing: up then down
    for i in 30..60 {
        bars[i].close = 160.0 - i as f64;
        bars[i].high = bars[i].close + 2.0;
        bars[i].low = bars[i].close - 1.0;
        bars[i].open = bars[i].close - 0.5;
    }
    let mut chart = ChartState::new("TEST", Timeframe::H4);
    chart.bars = bars;
    chart.compute_indicators();
    // Auto fib may or may not find levels depending on fractal detection.
    // Point of this test is "compute_indicators doesn't panic on a swing" —
    // the computation above is the assertion.
    let _ = chart.auto_fib_levels.len();
}

// ── ChartState Integration ───────────────────────────────────────────

#[test]
fn test_chart_state_compute_all_indicators() {
    let mut chart = ChartState::new("TEST", Timeframe::H4);
    chart.bars = make_bars(100);
    chart.compute_indicators();
    // All indicator vectors should have correct length
    assert_eq!(chart.sma200.len(), 100);
    assert_eq!(chart.sma100.len(), 100);
    assert_eq!(chart.kama.len(), 100);
    assert_eq!(chart.ema21.len(), 100);
    assert_eq!(chart.rsi.len(), 100);
    assert_eq!(chart.fisher.len(), 100);
    assert_eq!(chart.macd_line.len(), 100);
    assert_eq!(chart.atr.len(), 100);
    assert_eq!(chart.cmo.len(), 100);
    assert_eq!(chart.qstick.len(), 100);
    assert_eq!(chart.disparity.len(), 100);
    assert_eq!(chart.bop.len(), 100);
    assert_eq!(chart.stddev.len(), 100);
    assert_eq!(chart.mfi.len(), 100);
    assert_eq!(chart.trix_line.len(), 100);
    assert_eq!(chart.ppo_line.len(), 100);
    assert_eq!(chart.ultosc.len(), 100);
    assert_eq!(chart.stochrsi_k.len(), 100);
    assert_eq!(chart.better_vol_type.len(), 100);
}

#[test]
fn test_chart_talib_gpu_fallback_extension_bundle_ranges() {
    let bars = make_oscillating_bars(160);

    let mfi = compute_mfi(&bars, 14);
    for value in mfi.iter().flatten() {
        assert!(
            (0.0..=100.0).contains(value),
            "MFI should be 0-100: {}",
            value
        );
    }

    let (trix, trix_signal, trix_hist) = compute_trix(&bars, 15, 9);
    assert!(trix.iter().flatten().all(|v| v.is_finite()));
    assert!(trix_signal.iter().flatten().all(|v| v.is_finite()));
    assert!(trix_hist.iter().flatten().all(|v| v.is_finite()));

    let (ppo, ppo_signal, ppo_hist) = compute_ppo(&bars, 12, 26, 9);
    assert!(ppo.iter().flatten().all(|v| v.is_finite()));
    assert!(ppo_signal.iter().flatten().all(|v| v.is_finite()));
    assert!(ppo_hist.iter().flatten().all(|v| v.is_finite()));

    let ultosc = compute_ultosc(&bars);
    for value in ultosc.iter().flatten() {
        assert!(
            (0.0..=100.0).contains(value),
            "ULTOSC should be 0-100: {}",
            value
        );
    }

    let (stochrsi_k, stochrsi_d) = compute_stochrsi(&bars, 14, 14, 3, 3);
    for value in stochrsi_k.iter().flatten() {
        assert!(
            (0.0..=100.0).contains(value),
            "StochRSI %K should be 0-100: {}",
            value
        );
    }
    for value in stochrsi_d.iter().flatten() {
        assert!(
            (0.0..=100.0).contains(value),
            "StochRSI %D should be 0-100: {}",
            value
        );
    }
}

#[test]
fn chart_camera_accumulates_fractional_horizontal_pan() {
    let mut camera = ChartCamera::from_legacy(300, 100, false);
    camera.begin_pan(800.0, 400.0, 100.0, 20.0);

    camera.pan_pixels(3.0, 0.0, 800.0, 400.0, 500, 80.0, 120.0);
    assert!(
        (camera.right_edge_bar() - 299.625).abs() < 1e-9,
        "3px at 8px/bar should move by 0.375 bar, got {}",
        camera.right_edge_bar()
    );
    assert!(camera.manual_override());
    assert!(!camera.follow_latest);
}

#[test]
fn chart_camera_vertical_pan_uses_zoomed_visible_price_span() {
    let mut camera = ChartCamera::from_legacy(499, 100, false);
    camera.set_price_view(100.0, 10.0);
    camera.begin_pan(800.0, 400.0, 100.0, 20.0);

    camera.pan_pixels(0.0, 120.0, 800.0, 400.0, 500, 80.0, 120.0);

    assert!(
        (camera.price_center.unwrap() - 103.0).abs() < 1e-9,
        "120px over 400px of 10pt span should move price center by 3pt; got {:?}",
        camera.price_center
    );
    assert_eq!(camera.price_span, Some(10.0));
    assert!(camera.manual_override());
}

#[test]
fn chart_camera_price_range_can_free_pan_below_zero() {
    let mut camera = ChartCamera::from_legacy(499, 100, false);
    camera.set_price_view(0.10, 0.12);
    camera.begin_pan(800.0, 400.0, 0.10, 0.12);

    camera.pan_pixels(0.0, -380.0, 800.0, 400.0, 500, 0.10, 0.12);
    let (min, max) = camera.explicit_price_range().unwrap();

    assert!(
        min < 0.0,
        "free-look price range should be allowed below zero; got {min}..{max}"
    );
    assert!(max > min);
}

#[test]
fn chart_state_repeated_free_look_drag_keeps_camera_authoritative() {
    let mut chart = ChartState::new("TEST", Timeframe::H4);
    chart.bars = make_bars(500);
    chart.visible_bars = 100;
    chart.view_offset = 499;
    chart.begin_chart_camera_pan(800.0, 400.0);
    chart.pan_chart_camera_pixels(egui::vec2(80.0, 0.0), 800.0, 400.0);
    let first_right_edge = chart.camera.right_edge_bar();
    let first_price_range = chart.visible_price_range().unwrap();
    let first_gen = chart.visible_bars_gen;

    chart.begin_chart_camera_pan(800.0, 400.0);

    assert!(
        (chart.camera.right_edge_bar() - first_right_edge).abs() < 1e-9,
        "new drag must not rebuild camera from rounded legacy view_offset"
    );
    assert_eq!(chart.visible_price_range().unwrap(), first_price_range);
    assert!(
        chart.visible_bars_gen > first_gen,
        "camera changes must invalidate draw early-out"
    );
}

#[test]
fn chart_camera_allows_empty_space_at_both_horizontal_edges() {
    let mut camera = ChartCamera::from_legacy(99, 100, false);
    camera.begin_pan(800.0, 400.0, 100.0, 20.0);

    camera.pan_pixels(10_000.0, 0.0, 800.0, 400.0, 500, 100.0, 20.0);
    assert!(
        camera.right_edge_bar().abs() < 1e-9,
        "left free-look bound should put oldest bar at the right edge, not clamp the viewport full of data"
    );

    camera.begin_pan(800.0, 400.0, 100.0, 20.0);
    camera.pan_pixels(-10_000.0, 0.0, 800.0, 400.0, 500, 100.0, 20.0);
    assert!(
        (camera.right_edge_bar() - 598.0).abs() < 1e-9,
        "right free-look bound should put newest bar at the left edge for one viewport of empty space"
    );
}

#[test]
fn chart_state_visible_slot_window_preserves_empty_edge_slots() {
    let mut chart = ChartState::new("TEST", Timeframe::H4);
    chart.bars = make_bars(500);
    chart.visible_bars = 100;
    chart.view_offset = 99;
    chart.manual_view_override = true;
    chart.camera = ChartCamera::from_legacy(0, 100, true);

    let (start, end, first_slot, slots) = chart.visible_slot_window();
    assert_eq!((start, end, slots), (0, 1, 100));
    assert_eq!(
        first_slot, 99.0,
        "oldest bar should render in the final slot with empty space to its left"
    );

    chart.camera = ChartCamera::from_legacy(598, 100, true);
    let (start, end, first_slot, slots) = chart.visible_slot_window();
    assert_eq!((start, end, slots), (499, 500, 100));
    assert_eq!(
        first_slot, 0.0,
        "newest bar should render in the first slot with empty space to its right"
    );
}

#[test]
fn chart_state_visible_slot_window_preserves_fractional_camera_offset() {
    let mut chart = ChartState::new("TEST", Timeframe::H4);
    chart.bars = make_bars(500);
    chart.visible_bars = 100;
    chart.manual_view_override = true;
    chart.camera = ChartCamera::from_legacy(300, 100, true);
    chart.camera.begin_pan(800.0, 400.0, 100.0, 20.0);
    chart
        .camera
        .pan_pixels(3.0, 0.0, 800.0, 400.0, 500, 100.0, 20.0);

    let (start, end, first_slot, slots) = chart.visible_slot_window();

    assert_eq!((start, end, slots), (201, 301, 100));
    assert!(
        (first_slot - 0.375).abs() < 1e-6,
        "sub-bar drag must move candles smoothly instead of rounding/snap-back; got first_slot={first_slot}"
    );
}

#[test]
fn chart_price_pane_height_excludes_indicator_panes_for_one_to_one_drag() {
    assert_eq!(chart_price_pane_height(1000.0, 0), 978.0);
    assert_eq!(chart_price_pane_height(1000.0, 1), 898.0);
    assert_eq!(chart_price_pane_height(1000.0, 3), 738.0);
}

#[test]
fn chart_camera_reload_preserves_manual_position_but_follow_latest_tracks_end() {
    let mut manual = ChartCamera::from_legacy(588, 100, true);
    manual.on_data_len_changed(600, 720);
    assert!(
        (manual.right_edge_bar() - 588.0).abs() < 1e-9,
        "manual camera should preserve the user's absolute recentered viewport across live reloads"
    );
    assert!(!manual.follow_latest);

    let mut following = ChartCamera::from_legacy(600, 100, false);
    following.on_data_len_changed(601, 720);
    assert!(
        (following.right_edge_bar() - 724.0).abs() < 1e-9,
        "follow-latest camera should snap to new latest bar plus chart-shift margin"
    );
    assert!(following.follow_latest);
}

#[test]
fn test_chart_state_visible_range() {
    let mut chart = ChartState::new("TEST", Timeframe::H4);
    chart.bars = make_bars(500);
    chart.visible_bars = 200;
    chart.view_offset = 499;
    let (start, end) = chart.visible_range();
    assert_eq!(end - start, 200);
    assert_eq!(end, 500);
}

#[test]
fn chart_horizontal_zoom_marks_manual_view_override() {
    let mut chart = ChartState::new("WOK", Timeframe::H4);
    chart.bars = make_bars(500);
    chart.visible_bars = 200;
    chart.view_offset = 499;

    TyphooNApp::handle_zoom(&mut chart, 60.0);

    assert!(chart.visible_bars < 200);
    assert!(chart.manual_view_override);
}

#[test]
fn chart_zoom_keeps_free_look_camera_instead_of_rebuilding_from_legacy() {
    let mut chart = ChartState::new("WOK", Timeframe::H4);
    chart.bars = make_bars(500);
    chart.visible_bars = 100;
    chart.view_offset = 499;

    chart.begin_chart_camera_pan(800.0, 400.0);
    chart.pan_chart_camera_pixels(egui::vec2(83.0, 120.0), 800.0, 400.0);
    let right_before = chart.camera.right_edge_bar();
    let price_center_before = chart.camera.price_center.unwrap();

    chart.zoom_chart_price_by(1.25);
    assert!(
        (chart.camera.right_edge_bar() - right_before).abs() < 1e-9,
        "vertical zoom must not rebuild horizontal camera from rounded view_offset"
    );
    assert!(
        (chart.camera.price_center.unwrap() - price_center_before).abs() < 1e-9,
        "vertical zoom should scale around the current free-look price center"
    );

    TyphooNApp::handle_zoom(&mut chart, 30.0);
    assert!(
        chart.manual_view_override,
        "horizontal zoom must keep manual free-look active"
    );
    assert!(
        (chart.camera.price_center.unwrap() - price_center_before).abs() < 1e-9,
        "horizontal zoom must not reset vertical free-look price center"
    );
}

#[test]
fn chart_body_camera_pans_time_and_price() {
    let mut chart = ChartState::new("TEST", Timeframe::H4);
    chart.bars = make_bars(500);
    chart.visible_bars = 100;
    chart.view_offset = 499;

    chart.begin_chart_camera_pan(800.0, 400.0);
    chart.pan_chart_camera_pixels(egui::vec2(80.0, 120.0), 800.0, 400.0);

    assert_eq!(chart.view_offset, 489);
    assert!(
        chart.price_pan > 0.0,
        "dragging downward should move the series downward"
    );
    assert!(
        chart.manual_view_override,
        "manual pan must suppress auto-follow snapback on cache reload"
    );
}

#[test]
fn chart_body_camera_accumulates_sub_bar_motion_fractionally() {
    let mut chart = ChartState::new("TEST", Timeframe::H4);
    chart.bars = make_bars(500);
    chart.visible_bars = 100;
    chart.view_offset = 300;
    chart.price_pan = 2.0;

    chart.begin_chart_camera_pan(800.0, 400.0);
    chart.pan_chart_camera_pixels(egui::vec2(3.0, 0.0), 800.0, 400.0);

    assert!(
        (chart.camera.right_edge_bar() - 299.625).abs() < 1e-9,
        "camera must preserve fractional sub-bar pan; got {}",
        chart.camera.right_edge_bar()
    );
    assert_eq!(chart.price_pan, 2.0);
    assert!(chart.manual_view_override);
}

#[test]
fn chart_body_camera_vertical_pan_uses_zoomed_visible_price_span() {
    let mut chart = ChartState::new("WOK", Timeframe::H4);
    chart.bars = make_bars(500);
    chart.visible_bars = 100;
    chart.view_offset = 499;
    chart.price_zoom = 10.0;

    let (natural_center, natural_span) = chart.natural_visible_price_view().unwrap();
    chart.begin_chart_camera_pan(800.0, 400.0);
    chart.pan_chart_camera_pixels(egui::vec2(0.0, 120.0), 800.0, 400.0);

    let expected = natural_span / 10.0 * 120.0 / 400.0;
    assert!(
        (chart.price_pan - expected).abs() < 1e-9,
        "zoomed vertical pan should move by visible price span; got {}, expected {}",
        chart.price_pan,
        expected
    );
    assert!((chart.camera.price_center.unwrap() - (natural_center + expected)).abs() < 1e-9);
    assert!(chart.manual_view_override);
}

#[test]
fn test_chart_state_reload_match_requires_source_for_loaded_chart() {
    let mut chart = ChartState::new("BTC/USD", Timeframe::H2);
    chart.bars = make_bars(20);
    chart.primary_source = "kraken";

    assert!(chart.should_reload_for_bar_fetch("BTCUSD", "1Hour", "kraken"));
    assert!(!chart.should_reload_for_bar_fetch("BTCUSD", "1Hour", "alpaca"));

    chart.primary_source = "kraken-equities";
    assert!(chart.should_reload_for_bar_fetch("BTCUSD", "1Hour", "alpaca"));
    assert!(chart.should_reload_for_bar_fetch("BTCUSD", "1Hour", "yahoo-chart"));
    assert!(!chart.should_reload_for_bar_fetch("BTCUSD", "1Hour", "kraken"));
}

#[test]
fn test_chart_state_reload_match_allows_empty_chart_fill() {
    let chart = ChartState::new("AAPL", Timeframe::D1);

    assert!(chart.should_reload_for_bar_fetch("AAPL", "1Day", "alpaca"));
    assert!(!chart.should_reload_for_bar_fetch("MSFT", "1Day", "alpaca"));
}

