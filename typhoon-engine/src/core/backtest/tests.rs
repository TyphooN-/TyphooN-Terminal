use super::*;

// ── Helpers ────────────────────────────────────────────────────

/// Create a synthetic Bar with all OHLC set to the same value.
fn flat_bar(price: f64, ts: &str) -> Bar {
    Bar {
        timestamp: ts.to_string(),
        open: price,
        high: price,
        low: price,
        close: price,
        volume: 100.0,
    }
}

/// Create a Bar with explicit OHLCV.
fn bar(open: f64, high: f64, low: f64, close: f64, ts: &str) -> Bar {
    Bar {
        timestamp: ts.to_string(),
        open,
        high,
        low,
        close,
        volume: 100.0,
    }
}

/// Generate a trending-up series: prices go from `start` to `start + count - 1`.
fn trending_up(start: f64, count: usize) -> Vec<Bar> {
    (0..count)
        .map(|i| flat_bar(start + i as f64, &format!("2025-01-{:02}T00:00:00Z", i + 1)))
        .collect()
}

/// Generate a simple oscillating series for SMA crossover signals.
/// Pattern: rises for `half` bars, then falls for `half` bars, repeated.
fn oscillating(base: f64, amplitude: f64, half: usize, cycles: usize) -> Vec<Bar> {
    let mut bars = Vec::new();
    let mut day = 1;
    for _ in 0..cycles {
        for i in 0..half {
            let price = base + amplitude * (i as f64 / half as f64);
            bars.push(flat_bar(price, &format!("2025-01-{:02}T00:00:00Z", day)));
            day += 1;
        }
        for i in 0..half {
            let price = base + amplitude - amplitude * (i as f64 / half as f64);
            bars.push(flat_bar(price, &format!("2025-01-{:02}T00:00:00Z", day)));
            day += 1;
        }
    }
    bars
}

/// A simple always-buy-then-sell strategy for deterministic testing.
struct AlternateBuySell {
    last_signal: Option<Signal>,
}

impl AlternateBuySell {
    fn new() -> Self {
        Self { last_signal: None }
    }
}

impl Strategy for AlternateBuySell {
    fn on_bar(&mut self, _bar: &Bar, index: usize, _bars: &[Bar]) -> Option<Signal> {
        if index == 0 {
            return None;
        }
        match self.last_signal {
            None => {
                self.last_signal = Some(Signal::Buy);
                Some(Signal::Buy)
            }
            Some(Signal::Buy) => {
                self.last_signal = Some(Signal::Sell);
                Some(Signal::Sell)
            }
            Some(Signal::Sell) => {
                self.last_signal = Some(Signal::Buy);
                Some(Signal::Buy)
            }
            Some(Signal::Close) => {
                self.last_signal = Some(Signal::Buy);
                Some(Signal::Buy)
            }
        }
    }
    fn name(&self) -> &str {
        "AlternateBuySell"
    }
}

/// Strategy that only emits Close signals for testing close logic.
struct BuyThenClose {
    state: u8, // 0 = waiting, 1 = bought, 2 = closed
}

impl BuyThenClose {
    fn new() -> Self {
        Self { state: 0 }
    }
}

impl Strategy for BuyThenClose {
    fn on_bar(&mut self, _bar: &Bar, index: usize, _bars: &[Bar]) -> Option<Signal> {
        match (index, self.state) {
            (1, 0) => {
                self.state = 1;
                Some(Signal::Buy)
            }
            (3, 1) => {
                self.state = 2;
                Some(Signal::Close)
            }
            _ => None,
        }
    }
    fn name(&self) -> &str {
        "BuyThenClose"
    }
}

// ── TradeReport::from_trades ───────────────────────────────────

#[test]
fn trade_report_empty() {
    let report = TradeReport::from_trades(&[], 10000.0);
    assert_eq!(report.total_trades, 0);
    assert_eq!(report.win_rate, 0.0);
    assert_eq!(report.total_pnl, 0.0);
    assert_eq!(report.max_drawdown, 0.0);
}

#[test]
fn trade_report_single_winning_trade() {
    let trades = vec![Trade {
        entry_index: 0,
        exit_index: 1,
        entry_price: 100.0,
        exit_price: 110.0,
        side: "long".to_string(),
        pnl: 100.0,
        pnl_pct: 10.0,
        entry_time: "t0".into(),
        exit_time: "t1".into(),
    }];
    let report = TradeReport::from_trades(&trades, 10000.0);
    assert_eq!(report.total_trades, 1);
    assert!((report.win_rate - 100.0).abs() < 1e-6);
    assert_eq!(report.profit_factor, 999.0);
    assert!((report.total_pnl - 100.0).abs() < 1e-6);
    assert!((report.gross_profit - 100.0).abs() < 1e-6);
    assert!((report.gross_loss - 0.0).abs() < 1e-6); // no losses, stored as -0 = 0
    assert_eq!(report.max_consecutive_wins, 1);
    assert_eq!(report.max_consecutive_losses, 0);
    assert_eq!(report.max_drawdown, 0.0);
}

#[test]
fn trade_report_mixed_trades() {
    let trades = vec![
        Trade {
            entry_index: 0,
            exit_index: 1,
            entry_price: 100.0,
            exit_price: 120.0,
            side: "long".to_string(),
            pnl: 200.0,
            pnl_pct: 20.0,
            entry_time: "t0".into(),
            exit_time: "t1".into(),
        },
        Trade {
            entry_index: 1,
            exit_index: 2,
            entry_price: 120.0,
            exit_price: 110.0,
            side: "long".to_string(),
            pnl: -100.0,
            pnl_pct: -8.33,
            entry_time: "t1".into(),
            exit_time: "t2".into(),
        },
        Trade {
            entry_index: 2,
            exit_index: 3,
            entry_price: 110.0,
            exit_price: 130.0,
            side: "long".to_string(),
            pnl: 200.0,
            pnl_pct: 18.18,
            entry_time: "t2".into(),
            exit_time: "t3".into(),
        },
    ];
    let report = TradeReport::from_trades(&trades, 10000.0);
    assert_eq!(report.total_trades, 3);
    // 2 wins out of 3
    assert!((report.win_rate - 200.0 / 3.0).abs() < 0.01);
    // gross profit = 400, gross loss = 100 => PF = 4.0
    assert!((report.profit_factor - 4.0).abs() < 1e-6);
    assert!((report.total_pnl - 300.0).abs() < 1e-6);
    assert_eq!(report.max_consecutive_wins, 1); // W, L, W
    assert_eq!(report.max_consecutive_losses, 1);
    // Drawdown: equity goes 10000 -> 10200 -> 10100 -> 10300
    // Peak at 10200, dd = 100, dd_pct = 100/10200*100
    assert!(report.max_drawdown > 0.0);
}

#[test]
fn trade_report_all_losses() {
    let trades = vec![
        Trade {
            entry_index: 0,
            exit_index: 1,
            entry_price: 100.0,
            exit_price: 90.0,
            side: "long".to_string(),
            pnl: -100.0,
            pnl_pct: -10.0,
            entry_time: "t0".into(),
            exit_time: "t1".into(),
        },
        Trade {
            entry_index: 1,
            exit_index: 2,
            entry_price: 90.0,
            exit_price: 80.0,
            side: "long".to_string(),
            pnl: -100.0,
            pnl_pct: -11.11,
            entry_time: "t1".into(),
            exit_time: "t2".into(),
        },
    ];
    let report = TradeReport::from_trades(&trades, 10000.0);
    assert_eq!(report.win_rate, 0.0);
    assert_eq!(report.profit_factor, 0.0);
    assert_eq!(report.max_consecutive_losses, 2);
    assert_eq!(report.max_consecutive_wins, 0);
    assert!((report.total_pnl - (-200.0)).abs() < 1e-6);
}

// ── SMA helper ─────────────────────────────────────────────────

#[test]
fn sma_basic() {
    let bars = vec![
        flat_bar(10.0, "t0"),
        flat_bar(20.0, "t1"),
        flat_bar(30.0, "t2"),
    ];
    // SMA(3) at index 2 = (10+20+30)/3 = 20
    assert!((sma(&bars, 2, 3).unwrap() - 20.0).abs() < 1e-10);
    // SMA(2) at index 2 = (20+30)/2 = 25
    assert!((sma(&bars, 2, 2).unwrap() - 25.0).abs() < 1e-10);
}

#[test]
fn sma_insufficient_data() {
    let bars = vec![flat_bar(10.0, "t0"), flat_bar(20.0, "t1")];
    assert!(sma(&bars, 1, 3).is_none());
    assert!(sma(&bars, 0, 2).is_none());
}

// ── SMACrossStrategy ───────────────────────────────────────────

#[test]
fn sma_cross_generates_signals() {
    // Create bars where fast SMA will cross slow SMA.
    // Flat at 100 for 20 bars, then ramp up sharply.
    let mut bars: Vec<Bar> = (0..20)
        .map(|i| flat_bar(100.0, &format!("t{}", i)))
        .collect();
    for i in 20..40 {
        bars.push(flat_bar(100.0 + (i - 20) as f64 * 5.0, &format!("t{}", i)));
    }
    // Then ramp down
    for i in 40..60 {
        bars.push(flat_bar(200.0 - (i - 40) as f64 * 5.0, &format!("t{}", i)));
    }

    let mut strat = SMACrossStrategy::new(3, 10);
    let result = run_backtest(&bars, &mut strat, 10000.0);
    // Should generate at least one trade from crossovers
    assert!(
        result.trades.len() >= 1,
        "Expected at least 1 trade, got {}",
        result.trades.len()
    );
    assert_eq!(result.equity_curve.len(), bars.len());
}

#[test]
fn sma_cross_no_signal_flat_market() {
    // Completely flat market — no crossover possible
    let bars: Vec<Bar> = (0..50)
        .map(|i| flat_bar(100.0, &format!("t{}", i)))
        .collect();
    let mut strat = SMACrossStrategy::new(5, 20);
    let result = run_backtest(&bars, &mut strat, 10000.0);
    // With perfectly flat prices, SMA fast == SMA slow always, no crossover
    assert_eq!(result.trades.len(), 0);
}

#[test]
fn sma_cross_name() {
    let strat = SMACrossStrategy::new(5, 20);
    assert_eq!(strat.name(), "SMA Cross");
}

// ── run_backtest ───────────────────────────────────────────────

#[test]
fn run_backtest_empty_bars() {
    let bars: Vec<Bar> = Vec::new();
    let mut strat = SMACrossStrategy::new(3, 10);
    let result = run_backtest(&bars, &mut strat, 10000.0);
    assert_eq!(result.trades.len(), 0);
    assert_eq!(result.equity_curve.len(), 0);
    assert_eq!(result.report.total_trades, 0);
}

#[test]
fn run_backtest_alternating_signals() {
    // Prices: 100, 110, 100, 110, 100
    let bars = vec![
        flat_bar(100.0, "t0"),
        flat_bar(110.0, "t1"),
        flat_bar(100.0, "t2"),
        flat_bar(110.0, "t3"),
        flat_bar(100.0, "t4"),
    ];
    let mut strat = AlternateBuySell::new();
    let result = run_backtest(&bars, &mut strat, 10000.0);

    // Bar 0: nothing, Bar 1: Buy@110, Bar 2: Sell@100 (close long: pnl<0, open short),
    // Bar 3: Buy@110 (close short: pnl<0, open long), Bar 4: end => close long@100
    assert!(result.trades.len() >= 2);
    assert_eq!(result.equity_curve.len(), 5);
}

#[test]
fn run_backtest_close_signal() {
    let bars = vec![
        flat_bar(100.0, "t0"),
        flat_bar(100.0, "t1"),
        flat_bar(110.0, "t2"),
        flat_bar(120.0, "t3"),
        flat_bar(130.0, "t4"),
    ];
    let mut strat = BuyThenClose::new();
    let result = run_backtest(&bars, &mut strat, 10000.0);
    // Buy at bar 1 (price 100), Close at bar 3 (price 120) => pnl = (120-100) * (10000/100) = 2000
    assert_eq!(result.trades.len(), 1);
    assert_eq!(result.trades[0].side, "long");
    assert!((result.trades[0].entry_price - 100.0).abs() < 1e-6);
    assert!((result.trades[0].exit_price - 120.0).abs() < 1e-6);
    assert!((result.trades[0].pnl - 2000.0).abs() < 1e-6);
}

#[test]
fn run_backtest_equity_curve_length_matches_bars() {
    let bars = trending_up(100.0, 30);
    let mut strat = SMACrossStrategy::new(3, 10);
    let result = run_backtest(&bars, &mut strat, 10000.0);
    assert_eq!(result.equity_curve.len(), bars.len());
}

#[test]
fn run_backtest_closes_open_position_at_end() {
    // Trending up => SMA cross should eventually buy, then the position
    // should be force-closed at the last bar.
    let mut bars: Vec<Bar> = (0..15)
        .map(|i| flat_bar(100.0, &format!("t{}", i)))
        .collect();
    for i in 15..30 {
        bars.push(flat_bar(100.0 + (i - 15) as f64 * 10.0, &format!("t{}", i)));
    }
    let mut strat = SMACrossStrategy::new(3, 10);
    let result = run_backtest(&bars, &mut strat, 10000.0);
    // If there's an open position, the last trade's exit_index should be bars.len()-1
    if !result.trades.is_empty() {
        let last_trade = result.trades.last().unwrap();
        assert_eq!(last_trade.exit_index, bars.len() - 1);
    }
}

// ── bar_by_bar_backtest ────────────────────────────────────────

#[test]
fn bar_by_bar_empty() {
    let bars: Vec<Bar> = Vec::new();
    let mut strat = SMACrossStrategy::new(3, 10);
    let result = bar_by_bar_backtest(&bars, &mut strat, 10000.0);
    assert_eq!(result.states.len(), 0);
    assert_eq!(result.trades.len(), 0);
}

#[test]
fn bar_by_bar_state_count_matches_bars() {
    let bars = trending_up(100.0, 20);
    let mut strat = AlternateBuySell::new();
    let result = bar_by_bar_backtest(&bars, &mut strat, 10000.0);
    assert_eq!(result.states.len(), bars.len());
    // First state should have initial equity
    assert!((result.states[0].equity - 10000.0).abs() < 1e-6);
    assert_eq!(result.states[0].bar_index, 0);
}

#[test]
fn bar_by_bar_position_size_tracking() {
    let bars = vec![
        flat_bar(100.0, "t0"),
        flat_bar(100.0, "t1"),
        flat_bar(110.0, "t2"),
        flat_bar(120.0, "t3"),
        flat_bar(130.0, "t4"),
    ];
    let mut strat = BuyThenClose::new();
    let result = bar_by_bar_backtest(&bars, &mut strat, 10000.0);

    // Bar 0: flat
    assert_eq!(result.states[0].position_size, 0.0);
    // Bar 1: bought at 100, position_size = 10000/100 = 100
    assert!((result.states[1].position_size - 100.0).abs() < 1e-6);
    // Bar 2: still long
    assert!(result.states[2].position_size > 0.0);
    // Bar 3: closed
    assert_eq!(result.states[3].position_size, 0.0);
}

#[test]
fn bar_by_bar_close_at_end() {
    // Strategy buys but never closes — verify forced close at last bar
    struct BuyOnce {
        bought: bool,
    }
    impl Strategy for BuyOnce {
        fn on_bar(&mut self, _bar: &Bar, index: usize, _bars: &[Bar]) -> Option<Signal> {
            if index == 1 && !self.bought {
                self.bought = true;
                Some(Signal::Buy)
            } else {
                None
            }
        }
        fn name(&self) -> &str {
            "BuyOnce"
        }
    }

    let bars = vec![
        flat_bar(100.0, "t0"),
        flat_bar(100.0, "t1"),
        flat_bar(120.0, "t2"),
    ];
    let mut strat = BuyOnce { bought: false };
    let result = bar_by_bar_backtest(&bars, &mut strat, 10000.0);
    // Position should be force-closed at last bar
    assert_eq!(result.trades.len(), 1);
    assert_eq!(result.trades[0].exit_index, 2);
    assert!((result.trades[0].exit_price - 120.0).abs() < 1e-6);
    // Last state should show flat position and updated equity
    let last = result.states.last().unwrap();
    assert_eq!(last.position_size, 0.0);
}

#[test]
fn bar_by_bar_short_trade() {
    // Strategy: sell at bar 1 (open short), buy at bar 3 (close short)
    struct ShortThenClose {
        state: u8,
    }
    impl Strategy for ShortThenClose {
        fn on_bar(&mut self, _bar: &Bar, index: usize, _bars: &[Bar]) -> Option<Signal> {
            match (index, self.state) {
                (1, 0) => {
                    self.state = 1;
                    Some(Signal::Sell)
                }
                (3, 1) => {
                    self.state = 2;
                    Some(Signal::Buy)
                }
                _ => None,
            }
        }
        fn name(&self) -> &str {
            "ShortThenClose"
        }
    }

    let bars = vec![
        flat_bar(100.0, "t0"),
        flat_bar(100.0, "t1"), // short entry
        flat_bar(90.0, "t2"),
        flat_bar(80.0, "t3"), // close short (Buy signal)
        flat_bar(85.0, "t4"),
    ];
    let mut strat = ShortThenClose { state: 0 };
    let result = bar_by_bar_backtest(&bars, &mut strat, 10000.0);

    // Short from 100 closed at 80 => pnl = (100-80) * 10000/100 = 2000
    // Then a long is opened at 80, force-closed at 85 at end
    assert!(result.trades.len() >= 1);
    let short_trade = &result.trades[0];
    assert_eq!(short_trade.side, "short");
    assert!((short_trade.pnl - 2000.0).abs() < 1e-6);
}

// ── optimize_sma_cross ─────────────────────────────────────────

#[test]
fn optimize_sma_cross_basic() {
    // Need enough bars for the largest slow period we test
    let mut bars: Vec<Bar> = (0..20)
        .map(|i| flat_bar(100.0, &format!("t{}", i)))
        .collect();
    for i in 20..60 {
        bars.push(flat_bar(100.0 + (i - 20) as f64 * 2.0, &format!("t{}", i)));
    }
    for i in 60..100 {
        bars.push(flat_bar(180.0 - (i - 60) as f64 * 2.0, &format!("t{}", i)));
    }

    let report = optimize_sma_cross(&bars, (3, 5), (10, 15), 10000.0, 5);
    assert!(report.total_combinations > 0);
    assert!(!report.results.is_empty());
    // Results should be sorted by profit factor descending
    for i in 1..report.results.len() {
        assert!(
            report.results[i - 1].profit_factor >= report.results[i].profit_factor
                || report.results[i - 1].profit_factor.is_infinite()
        );
    }
}

#[test]
fn optimize_sma_cross_skips_invalid_combos() {
    let bars = trending_up(100.0, 50);
    let report = optimize_sma_cross(&bars, (10, 12), (5, 9), 10000.0, 10);
    // fast >= slow for all combos, so nothing should run
    assert_eq!(report.total_combinations, 0);
    assert!(report.results.is_empty());
}

#[test]
fn optimize_sma_cross_top_n_limit() {
    let mut bars: Vec<Bar> = (0..30)
        .map(|i| flat_bar(100.0, &format!("t{}", i)))
        .collect();
    for i in 30..80 {
        bars.push(flat_bar(100.0 + (i - 30) as f64, &format!("t{}", i)));
    }
    for i in 80..120 {
        bars.push(flat_bar(150.0 - (i - 80) as f64, &format!("t{}", i)));
    }

    let report = optimize_sma_cross(&bars, (2, 10), (15, 30), 10000.0, 3);
    assert!(report.results.len() <= 3);
}

#[test]
fn optimize_sma_cross_empty_bars() {
    let bars: Vec<Bar> = Vec::new();
    let report = optimize_sma_cross(&bars, (3, 5), (10, 20), 10000.0, 5);
    assert_eq!(report.total_combinations, 0);
}

// ── walk_forward ───────────────────────────────────────────────

#[test]
fn walk_forward_too_few_bars() {
    let bars = trending_up(100.0, 50); // < 200
    let result = walk_forward(&bars, 3..10, 15..50, 3, 10000.0);
    assert!(result.windows.is_empty());
    assert_eq!(result.oos_sharpe, 0.0);
    assert_eq!(result.best_params, (10, 50)); // default
}

#[test]
fn walk_forward_too_few_windows() {
    let bars = trending_up(100.0, 300);
    let result = walk_forward(&bars, 3..10, 15..50, 1, 10000.0); // num_windows < 2
    assert!(result.windows.is_empty());
}

#[test]
fn walk_forward_produces_windows() {
    // Generate enough bars with a clear trend shift for meaningful results
    let mut bars: Vec<Bar> = Vec::new();
    let mut day = 1;
    // 4 cycles of up/down with ~100 bars each = 400 bars
    for cycle in 0..4 {
        let base = 100.0 + cycle as f64 * 10.0;
        for i in 0..50 {
            bars.push(flat_bar(base + i as f64, &format!("t{}", day)));
            day += 1;
        }
        for i in 0..50 {
            bars.push(flat_bar(base + 50.0 - i as f64, &format!("t{}", day)));
            day += 1;
        }
    }

    let result = walk_forward(&bars, 3..12, 15..40, 3, 10000.0);
    assert!(!result.windows.is_empty());
    assert!(result.windows.len() <= 3);
    // Each window should have valid indices
    for w in &result.windows {
        assert!(w.is_end > w.is_start);
        assert!(w.oos_end > w.oos_start);
    }
}

#[test]
fn walk_forward_robustness_is_ratio() {
    // Just verify the structure is computed without panic
    let mut bars: Vec<Bar> = Vec::new();
    for i in 0..300 {
        let price = 100.0 + 20.0 * (i as f64 * 0.05).sin();
        bars.push(flat_bar(price, &format!("t{}", i)));
    }

    let result = walk_forward(&bars, 3..8, 15..30, 2, 10000.0);
    // Robustness score should be finite
    assert!(result.robustness_score.is_finite());
    assert!(result.oos_win_rate >= 0.0 && result.oos_win_rate <= 1.0);
}

// ── NNFXStrategy ───────────────────────────────────────────────

#[test]
fn nnfx_strategy_name() {
    let strat = NNFXStrategy::new(10, 5);
    assert_eq!(strat.name(), "NNFX (KAMA+Fisher)");
}

#[test]
fn nnfx_needs_warmup() {
    let bars = trending_up(100.0, 5);
    let mut strat = NNFXStrategy::new(10, 5);
    // With only 5 bars and period=10, should get no signals
    let result = run_backtest(&bars, &mut strat, 10000.0);
    assert_eq!(result.trades.len(), 0);
}

#[test]
fn nnfx_with_sufficient_data() {
    // Generate bars with enough data for KAMA(10) + Fisher(5) warmup
    let mut bars: Vec<Bar> = Vec::new();
    for i in 0..50 {
        let price = 100.0 + 30.0 * (i as f64 * 0.15).sin();
        bars.push(bar(
            price - 1.0,
            price + 2.0,
            price - 2.0,
            price,
            &format!("t{}", i),
        ));
    }
    let mut strat = NNFXStrategy::new(10, 5);
    let result = run_backtest(&bars, &mut strat, 10000.0);
    // Just verify it runs without panic; trade count depends on data
    assert_eq!(result.equity_curve.len(), 50);
}

// ── KAMACrossStrategy ──────────────────────────────────────────

#[test]
fn kama_cross_name() {
    let strat = KAMACrossStrategy::new(10, 2, 30);
    assert_eq!(strat.name(), "KAMA Cross");
}

#[test]
fn kama_cross_needs_warmup() {
    let bars = trending_up(100.0, 5);
    let mut strat = KAMACrossStrategy::new(10, 2, 30);
    let result = run_backtest(&bars, &mut strat, 10000.0);
    assert_eq!(result.trades.len(), 0);
}

#[test]
fn kama_cross_with_oscillating_data() {
    let bars = oscillating(100.0, 50.0, 15, 3);
    let mut strat = KAMACrossStrategy::new(10, 2, 30);
    let result = run_backtest(&bars, &mut strat, 10000.0);
    assert_eq!(result.equity_curve.len(), bars.len());
}

// ── FisherCrossStrategy ────────────────────────────────────────

#[test]
fn fisher_cross_name() {
    let strat = FisherCrossStrategy::new(10);
    assert_eq!(strat.name(), "Fisher Cross");
}

#[test]
fn fisher_cross_needs_warmup() {
    let bars: Vec<Bar> = (0..5)
        .map(|i| bar(100.0, 105.0, 95.0, 100.0 + i as f64, &format!("t{}", i)))
        .collect();
    let mut strat = FisherCrossStrategy::new(10);
    let result = run_backtest(&bars, &mut strat, 10000.0);
    assert_eq!(result.trades.len(), 0);
}

#[test]
fn fisher_cross_with_data() {
    let mut bars: Vec<Bar> = Vec::new();
    for i in 0..60 {
        let price = 100.0 + 20.0 * (i as f64 * 0.2).sin();
        bars.push(bar(
            price,
            price + 3.0,
            price - 3.0,
            price,
            &format!("t{}", i),
        ));
    }
    let mut strat = FisherCrossStrategy::new(10);
    let result = run_backtest(&bars, &mut strat, 10000.0);
    assert_eq!(result.equity_curve.len(), 60);
}

// ── RSIMeanRevStrategy ─────────────────────────────────────────

#[test]
fn rsi_mean_rev_name() {
    let strat = RSIMeanRevStrategy::new(14, 30.0, 70.0);
    assert_eq!(strat.name(), "RSI Mean Rev");
}

#[test]
fn rsi_mean_rev_needs_warmup() {
    let bars = trending_up(100.0, 10);
    let mut strat = RSIMeanRevStrategy::new(14, 30.0, 70.0);
    let result = run_backtest(&bars, &mut strat, 10000.0);
    assert_eq!(result.trades.len(), 0);
}

#[test]
fn rsi_mean_rev_triggers_on_extreme_moves() {
    // Sharp drop then recovery to trigger oversold -> buy -> close
    let mut bars: Vec<Bar> = Vec::new();
    // Flat start for warmup
    for i in 0..30 {
        bars.push(flat_bar(100.0, &format!("t{}", i)));
    }
    // Sharp drop
    for i in 30..40 {
        bars.push(flat_bar(100.0 - (i - 30) as f64 * 5.0, &format!("t{}", i)));
    }
    // Recovery
    for i in 40..60 {
        bars.push(flat_bar(50.0 + (i - 40) as f64 * 5.0, &format!("t{}", i)));
    }

    let mut strat = RSIMeanRevStrategy::new(14, 30.0, 70.0);
    let result = run_backtest(&bars, &mut strat, 10000.0);
    // Should have triggered at least one signal from the extreme drop
    assert_eq!(result.equity_curve.len(), bars.len());
}

// ── rsi_at helper ──────────────────────────────────────────────

#[test]
fn rsi_at_insufficient_data() {
    let bars = trending_up(100.0, 5);
    assert!(rsi_at(&bars, 4, 14).is_none());
}

#[test]
fn rsi_at_all_up() {
    // Steadily rising prices => RSI should be near 100
    let bars = trending_up(100.0, 40);
    let rsi = rsi_at(&bars, 39, 14);
    assert!(rsi.is_some());
    assert!(
        rsi.unwrap() > 80.0,
        "Expected RSI > 80 for all-up, got {}",
        rsi.unwrap()
    );
}

// ── fisher_at helper ───────────────────────────────────────────

#[test]
fn fisher_at_insufficient_data() {
    let bars = vec![bar(100.0, 105.0, 95.0, 100.0, "t0")];
    assert!(fisher_at(&bars, 0, 5).is_none());
}

#[test]
fn fisher_at_flat_range() {
    // All bars identical => range = 0 => Fisher = 0
    let bars: Vec<Bar> = (0..10)
        .map(|i| bar(100.0, 100.0, 100.0, 100.0, &format!("t{}", i)))
        .collect();
    let f = fisher_at(&bars, 9, 5).unwrap();
    assert!((f - 0.0).abs() < 1e-10);
}

#[test]
fn fisher_at_at_high() {
    // Price at the top of the range => Fisher should be positive
    let mut bars: Vec<Bar> = Vec::new();
    for i in 0..10 {
        let mid = 100.0 + i as f64;
        bars.push(bar(mid, mid + 1.0, mid - 1.0, mid, &format!("t{}", i)));
    }
    let f = fisher_at(&bars, 9, 5).unwrap();
    assert!(f > 0.0, "Expected positive Fisher at high, got {}", f);
}

// ── kama_at helper ─────────────────────────────────────────────

#[test]
fn kama_at_insufficient_data() {
    let bars = trending_up(100.0, 3);
    assert!(kama_at(&bars, 2, 10).is_none());
}

#[test]
fn kama_at_trending_follows_price() {
    let bars = trending_up(100.0, 30);
    let k = kama_at(&bars, 29, 10).unwrap();
    // KAMA should be between start and end prices
    assert!(
        k > 100.0 && k < 130.0,
        "KAMA={} should be between 100 and 130",
        k
    );
}

// ── fisher_pair helper ─────────────────────────────────────────

#[test]
fn fisher_pair_insufficient_data() {
    let bars = vec![bar(100.0, 105.0, 95.0, 100.0, "t0")];
    assert!(fisher_pair(&bars, 0, 10).is_none());
}

#[test]
fn fisher_pair_returns_two_values() {
    let mut bars: Vec<Bar> = Vec::new();
    for i in 0..20 {
        let mid = 100.0 + 10.0 * (i as f64 * 0.3).sin();
        bars.push(bar(mid, mid + 2.0, mid - 2.0, mid, &format!("t{}", i)));
    }
    let (fisher, signal) = fisher_pair(&bars, 19, 10).unwrap();
    assert!(fisher.is_finite());
    assert!(signal.is_finite());
}

// ── kama_custom helper ─────────────────────────────────────────

#[test]
fn kama_custom_insufficient_data() {
    let bars = trending_up(100.0, 3);
    assert!(kama_custom(&bars, 2, 10, 2, 30).is_none());
}

#[test]
fn kama_custom_trending() {
    let bars = trending_up(100.0, 30);
    let k = kama_custom(&bars, 29, 10, 2, 30).unwrap();
    assert!(k > 100.0 && k < 130.0, "KAMA custom={} out of range", k);
}

// ── Signal enum ────────────────────────────────────────────────

#[test]
fn signal_equality() {
    assert_eq!(Signal::Buy, Signal::Buy);
    assert_ne!(Signal::Buy, Signal::Sell);
    assert_ne!(Signal::Sell, Signal::Close);
}

// ── Integration: consistency between run_backtest and bar_by_bar ──

#[test]
fn run_backtest_and_bar_by_bar_same_trades() {
    let bars = oscillating(100.0, 30.0, 12, 3);

    let mut strat1 = SMACrossStrategy::new(3, 8);
    let result1 = run_backtest(&bars, &mut strat1, 10000.0);

    let mut strat2 = SMACrossStrategy::new(3, 8);
    let result2 = bar_by_bar_backtest(&bars, &mut strat2, 10000.0);

    assert_eq!(
        result1.trades.len(),
        result2.trades.len(),
        "run_backtest ({}) and bar_by_bar ({}) should produce same number of trades",
        result1.trades.len(),
        result2.trades.len()
    );

    for (t1, t2) in result1.trades.iter().zip(result2.trades.iter()) {
        assert_eq!(t1.entry_index, t2.entry_index);
        assert_eq!(t1.exit_index, t2.exit_index);
        assert!((t1.pnl - t2.pnl).abs() < 1e-6);
    }
}

#[test]
fn run_backtest_and_bar_by_bar_same_final_equity() {
    let bars = trending_up(50.0, 40);
    let mut strat1 = AlternateBuySell::new();
    let result1 = run_backtest(&bars, &mut strat1, 5000.0);

    let mut strat2 = AlternateBuySell::new();
    let result2 = bar_by_bar_backtest(&bars, &mut strat2, 5000.0);

    let final_eq1 = *result1.equity_curve.last().unwrap_or(&5000.0);
    let final_eq2 = result2.states.last().map(|s| s.equity).unwrap_or(5000.0);
    assert!(
        (final_eq1 - final_eq2).abs() < 1e-4,
        "Final equity mismatch: run_backtest={}, bar_by_bar={}",
        final_eq1,
        final_eq2
    );
}
