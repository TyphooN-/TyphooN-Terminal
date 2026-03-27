//! Strategy backtester — run strategies against historical bar data.
//!
//! Provides a `Strategy` trait, a simple SMA-cross example, and backtest
//! engine that produces equity curves, trade logs, and performance metrics.

use crate::broker::alpaca::Bar;
use serde::{Deserialize, Serialize};

// ── Signal & Trade Types ────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Signal {
    Buy,
    Sell,
    Close,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Trade {
    pub entry_index: usize,
    pub exit_index: usize,
    pub entry_price: f64,
    pub exit_price: f64,
    pub side: String, // "long" or "short"
    pub pnl: f64,
    pub pnl_pct: f64,
    pub entry_time: String,
    pub exit_time: String,
}

// ── Trade Report ────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TradeReport {
    pub total_trades: usize,
    pub win_rate: f64,
    pub profit_factor: f64,
    pub sharpe_ratio: f64,
    pub max_drawdown: f64,
    pub max_drawdown_pct: f64,
    pub max_consecutive_wins: u32,
    pub max_consecutive_losses: u32,
    pub avg_win: f64,
    pub avg_loss: f64,
    pub avg_trade: f64,
    pub total_pnl: f64,
    pub gross_profit: f64,
    pub gross_loss: f64,
}

impl TradeReport {
    pub fn from_trades(trades: &[Trade], initial_equity: f64) -> Self {
        if trades.is_empty() {
            return Self {
                total_trades: 0,
                win_rate: 0.0,
                profit_factor: 0.0,
                sharpe_ratio: 0.0,
                max_drawdown: 0.0,
                max_drawdown_pct: 0.0,
                max_consecutive_wins: 0,
                max_consecutive_losses: 0,
                avg_win: 0.0,
                avg_loss: 0.0,
                avg_trade: 0.0,
                total_pnl: 0.0,
                gross_profit: 0.0,
                gross_loss: 0.0,
            };
        }

        let wins: Vec<f64> = trades.iter().filter(|t| t.pnl > 0.0).map(|t| t.pnl).collect();
        let losses: Vec<f64> = trades.iter().filter(|t| t.pnl <= 0.0).map(|t| t.pnl).collect();

        let gross_profit: f64 = wins.iter().sum();
        let gross_loss: f64 = losses.iter().map(|l| l.abs()).sum();
        let total_pnl: f64 = trades.iter().map(|t| t.pnl).sum();

        let win_rate = wins.len() as f64 / trades.len() as f64 * 100.0;
        let profit_factor = if gross_loss > 0.0 { gross_profit / gross_loss } else { f64::INFINITY };
        let avg_win = if wins.is_empty() { 0.0 } else { gross_profit / wins.len() as f64 };
        let avg_loss = if losses.is_empty() { 0.0 } else { gross_loss / losses.len() as f64 };
        let avg_trade = total_pnl / trades.len() as f64;

        // Max consecutive wins/losses
        let mut max_con_wins: u32 = 0;
        let mut max_con_losses: u32 = 0;
        let mut cur_wins: u32 = 0;
        let mut cur_losses: u32 = 0;
        for t in trades {
            if t.pnl > 0.0 {
                cur_wins += 1;
                cur_losses = 0;
                max_con_wins = max_con_wins.max(cur_wins);
            } else {
                cur_losses += 1;
                cur_wins = 0;
                max_con_losses = max_con_losses.max(cur_losses);
            }
        }

        // Max drawdown from equity curve
        let mut equity = initial_equity;
        let mut peak = equity;
        let mut max_dd = 0.0_f64;
        let mut max_dd_pct = 0.0_f64;
        for t in trades {
            equity += t.pnl;
            peak = peak.max(equity);
            let dd = peak - equity;
            let dd_pct = if peak > 0.0 { dd / peak * 100.0 } else { 0.0 };
            max_dd = max_dd.max(dd);
            max_dd_pct = max_dd_pct.max(dd_pct);
        }

        // Sharpe ratio (annualized, assuming daily returns)
        let returns: Vec<f64> = trades.iter().map(|t| t.pnl_pct / 100.0).collect();
        let mean_ret = returns.iter().sum::<f64>() / returns.len() as f64;
        let variance = returns.iter().map(|r| (r - mean_ret).powi(2)).sum::<f64>() / returns.len() as f64;
        let std_dev = variance.sqrt();
        let sharpe = if std_dev > 1e-10 { (mean_ret / std_dev) * (252.0_f64).sqrt() } else { 0.0 };

        Self {
            total_trades: trades.len(),
            win_rate,
            profit_factor,
            sharpe_ratio: sharpe,
            max_drawdown: max_dd,
            max_drawdown_pct: max_dd_pct,
            max_consecutive_wins: max_con_wins,
            max_consecutive_losses: max_con_losses,
            avg_win,
            avg_loss,
            avg_trade,
            total_pnl,
            gross_profit,
            gross_loss: -gross_loss, // return as negative
        }
    }
}

// ── Backtest Result ─────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BacktestResult {
    pub trades: Vec<Trade>,
    pub equity_curve: Vec<f64>,
    pub report: TradeReport,
}

// ── Strategy Trait ──────────────────────────────────────────────────

pub trait Strategy: Send {
    fn on_bar(&mut self, bar: &Bar, index: usize, bars: &[Bar]) -> Option<Signal>;
    fn name(&self) -> &str;
}

// ── SMA Cross Strategy ─────────────────────────────────────────────

pub struct SMACrossStrategy {
    pub fast_period: usize,
    pub slow_period: usize,
}

impl SMACrossStrategy {
    pub fn new(fast_period: usize, slow_period: usize) -> Self {
        Self { fast_period, slow_period }
    }
}

fn sma(bars: &[Bar], end: usize, period: usize) -> Option<f64> {
    if end + 1 < period {
        return None;
    }
    let start = end + 1 - period;
    let sum: f64 = bars[start..=end].iter().map(|b| b.close).sum();
    Some(sum / period as f64)
}

impl Strategy for SMACrossStrategy {
    fn on_bar(&mut self, _bar: &Bar, index: usize, bars: &[Bar]) -> Option<Signal> {
        if index < 1 { return None; }

        let fast_now = sma(bars, index, self.fast_period)?;
        let slow_now = sma(bars, index, self.slow_period)?;
        let fast_prev = sma(bars, index - 1, self.fast_period)?;
        let slow_prev = sma(bars, index - 1, self.slow_period)?;

        // Crossover: fast crosses above slow → Buy
        if fast_prev <= slow_prev && fast_now > slow_now {
            return Some(Signal::Buy);
        }
        // Crossunder: fast crosses below slow → Sell
        if fast_prev >= slow_prev && fast_now < slow_now {
            return Some(Signal::Sell);
        }
        None
    }

    fn name(&self) -> &str {
        "SMA Cross"
    }
}

// ── NNFX Strategy (KAMA + Fisher Transform) ──────────────────────────
//
// Entry: KAMA crosses price (trend) + Fisher confirms direction
// Exit: Opposite signal or Fisher reversal
// Uses the same NNFX indicators as the main chart system.

pub struct NNFXStrategy {
    pub kama_period: usize,
    pub fisher_period: usize,
    kama_prev: f64,
    fisher_prev: f64,
}

impl NNFXStrategy {
    pub fn new(kama_period: usize, fisher_period: usize) -> Self {
        Self { kama_period, fisher_period, kama_prev: 0.0, fisher_prev: 0.0 }
    }
}

fn kama_at(bars: &[Bar], end: usize, period: usize) -> Option<f64> {
    if end + 1 < period + 1 { return None; }
    let fast_sc = 2.0 / 3.0;  // fast=2
    let slow_sc = 2.0 / 31.0; // slow=30
    let start = end + 1 - (period + 1);
    let mut kama = bars[start].open;
    for i in (start + 1)..=end {
        let direction = (bars[i].open - bars[i.saturating_sub(period)].open).abs();
        let mut volatility = 0.0;
        for j in (i.saturating_sub(period - 1))..=i {
            if j > 0 { volatility += (bars[j].open - bars[j - 1].open).abs(); }
        }
        let er = if volatility > 1e-10 { direction / volatility } else { 0.0 };
        let sc = (er * (fast_sc - slow_sc) + slow_sc).powi(2);
        kama += sc * (bars[i].open - kama);
    }
    Some(kama)
}

fn fisher_at(bars: &[Bar], end: usize, period: usize) -> Option<f64> {
    if end + 1 < period { return None; }
    let start = end + 1 - period;
    let mut highest = f64::MIN;
    let mut lowest = f64::MAX;
    for i in start..=end {
        let mid = (bars[i].high + bars[i].low) / 2.0;
        highest = highest.max(mid);
        lowest = lowest.min(mid);
    }
    let range = highest - lowest;
    if range < 1e-10 { return Some(0.0); }
    let mid = (bars[end].high + bars[end].low) / 2.0;
    let raw = 2.0 * ((mid - lowest) / range - 0.5);
    let clamped = raw.clamp(-0.999, 0.999);
    Some(0.5 * ((1.0 + clamped) / (1.0 - clamped)).ln())
}

impl Strategy for NNFXStrategy {
    fn on_bar(&mut self, _bar: &Bar, index: usize, bars: &[Bar]) -> Option<Signal> {
        let kama_now = kama_at(bars, index, self.kama_period)?;
        let fisher_now = fisher_at(bars, index, self.fisher_period)?;

        if index < 1 || self.kama_prev == 0.0 {
            self.kama_prev = kama_now;
            self.fisher_prev = fisher_now;
            return None;
        }

        let price = bars[index].close;
        let prev_price = bars[index - 1].close;
        let signal = if prev_price <= self.kama_prev && price > kama_now && fisher_now > 0.0 {
            // Price crosses above KAMA + Fisher bullish → Buy
            Some(Signal::Buy)
        } else if prev_price >= self.kama_prev && price < kama_now && fisher_now < 0.0 {
            // Price crosses below KAMA + Fisher bearish → Sell
            Some(Signal::Sell)
        } else {
            None
        };

        self.kama_prev = kama_now;
        self.fisher_prev = fisher_now;
        signal
    }

    fn name(&self) -> &str {
        "NNFX (KAMA+Fisher)"
    }
}

// ── KAMA Cross Strategy ───────────────────────────────────────────

/// KAMA crossover strategy — buy when price crosses above KAMA, sell when below.
pub struct KAMACrossStrategy {
    period: usize,
    fast: usize,
    slow: usize,
    position: i8, // 0 = flat, 1 = long, -1 = short
}

impl KAMACrossStrategy {
    pub fn new(period: usize, fast: usize, slow: usize) -> Self {
        Self { period, fast, slow, position: 0 }
    }
}

fn kama_custom(bars: &[Bar], end: usize, period: usize, fast: usize, slow: usize) -> Option<f64> {
    if end + 1 < period + 1 { return None; }
    let fast_sc = 2.0 / (fast as f64 + 1.0);
    let slow_sc = 2.0 / (slow as f64 + 1.0);
    let start = end + 1 - (period + 1);
    let mut kama = bars[start].close;
    for i in (start + 1)..=end {
        let direction = (bars[i].close - bars[i.saturating_sub(period)].close).abs();
        let mut volatility = 0.0;
        for j in (i.saturating_sub(period - 1))..=i {
            if j > 0 { volatility += (bars[j].close - bars[j - 1].close).abs(); }
        }
        let er = if volatility > 1e-10 { direction / volatility } else { 0.0 };
        let sc = (er * (fast_sc - slow_sc) + slow_sc).powi(2);
        kama += sc * (bars[i].close - kama);
    }
    Some(kama)
}

impl Strategy for KAMACrossStrategy {
    fn on_bar(&mut self, _bar: &Bar, index: usize, bars: &[Bar]) -> Option<Signal> {
        if index < 1 { return None; }
        let kama_now = kama_custom(bars, index, self.period, self.fast, self.slow)?;
        let kama_prev = kama_custom(bars, index - 1, self.period, self.fast, self.slow)?;
        let close = bars[index].close;
        let prev_close = bars[index - 1].close;

        // Price crosses above KAMA → Buy
        if prev_close <= kama_prev && close > kama_now && self.position != 1 {
            self.position = 1;
            return Some(Signal::Buy);
        }
        // Price crosses below KAMA → Sell
        if prev_close >= kama_prev && close < kama_now && self.position != -1 {
            self.position = -1;
            return Some(Signal::Sell);
        }
        None
    }

    fn name(&self) -> &str {
        "KAMA Cross"
    }
}

// ── Fisher Cross Strategy ─────────────────────────────────────────

/// Fisher Transform crossover — buy when Fisher > Signal, sell when Fisher < Signal.
pub struct FisherCrossStrategy {
    period: usize,
    position: i8,
}

impl FisherCrossStrategy {
    pub fn new(period: usize) -> Self {
        Self { period, position: 0 }
    }
}

/// Compute Fisher Transform value at given bar index. Returns (fisher, signal).
fn fisher_pair(bars: &[Bar], end: usize, period: usize) -> Option<(f64, f64)> {
    if end + 1 < period + 1 { return None; }
    // We need at least period+1 bars to compute both current and previous Fisher
    let start = end + 1 - (period + 1).min(end + 1);
    let mut val = 0.0_f64;
    let mut prev_val = 0.0_f64;
    let mut fisher = 0.0_f64;
    let mut prev_fisher = 0.0_f64;

    // Walk through bars to iteratively compute Fisher
    for i in start..=end {
        if i + 1 < period { continue; }
        let lo = i + 1 - period;
        let mut highest = f64::MIN;
        let mut lowest = f64::MAX;
        for j in lo..=i {
            let mid = (bars[j].high + bars[j].low) / 2.0;
            highest = highest.max(mid);
            lowest = lowest.min(mid);
        }
        let range = highest - lowest;
        let mid = (bars[i].high + bars[i].low) / 2.0;
        let raw = if range > 1e-10 { 2.0 * ((mid - lowest) / range - 0.5) } else { 0.0 };
        val = 0.5 * val + 0.5 * raw.clamp(-0.999, 0.999);
        prev_fisher = fisher;
        fisher = 0.5 * ((1.0 + val) / (1.0 - val)).ln() + 0.5 * prev_fisher;
        prev_val = val;
    }
    let _ = prev_val; // suppress unused warning
    Some((fisher, prev_fisher))
}

impl Strategy for FisherCrossStrategy {
    fn on_bar(&mut self, _bar: &Bar, index: usize, bars: &[Bar]) -> Option<Signal> {
        if index < 1 { return None; }
        let (fisher_now, signal_now) = fisher_pair(bars, index, self.period)?;
        let (fisher_prev, signal_prev) = fisher_pair(bars, index - 1, self.period)?;

        // Fisher crosses above Signal → Buy
        if fisher_prev <= signal_prev && fisher_now > signal_now && self.position != 1 {
            self.position = 1;
            return Some(Signal::Buy);
        }
        // Fisher crosses below Signal → Sell
        if fisher_prev >= signal_prev && fisher_now < signal_now && self.position != -1 {
            self.position = -1;
            return Some(Signal::Sell);
        }
        None
    }

    fn name(&self) -> &str {
        "Fisher Cross"
    }
}

// ── RSI Mean Reversion Strategy ───────────────────────────────────

/// RSI mean-reversion — buy when RSI < oversold, sell when RSI > overbought.
pub struct RSIMeanRevStrategy {
    period: usize,
    oversold: f64,
    overbought: f64,
    position: i8,
}

impl RSIMeanRevStrategy {
    pub fn new(period: usize, oversold: f64, overbought: f64) -> Self {
        Self { period, oversold, overbought, position: 0 }
    }
}

fn rsi_at(bars: &[Bar], end: usize, period: usize) -> Option<f64> {
    if end < period { return None; }
    let mut avg_gain = 0.0;
    let mut avg_loss = 0.0;

    // Wilder-smoothed RSI
    let seed_start = if end + 1 > 2 * period { end + 1 - 2 * period } else { 1 };
    let seed_end = seed_start + period;
    if seed_end > end + 1 || seed_start == 0 { return None; }

    for i in seed_start..seed_end {
        let change = bars[i].close - bars[i - 1].close;
        if change > 0.0 { avg_gain += change; } else { avg_loss += change.abs(); }
    }
    avg_gain /= period as f64;
    avg_loss /= period as f64;

    // Smooth forward
    for i in seed_end..=end {
        let change = bars[i].close - bars[i - 1].close;
        let gain = if change > 0.0 { change } else { 0.0 };
        let loss = if change < 0.0 { change.abs() } else { 0.0 };
        avg_gain = (avg_gain * (period as f64 - 1.0) + gain) / period as f64;
        avg_loss = (avg_loss * (period as f64 - 1.0) + loss) / period as f64;
    }

    if avg_loss < 1e-10 { return Some(100.0); }
    let rs = avg_gain / avg_loss;
    Some(100.0 - 100.0 / (1.0 + rs))
}

impl Strategy for RSIMeanRevStrategy {
    fn on_bar(&mut self, _bar: &Bar, index: usize, bars: &[Bar]) -> Option<Signal> {
        let rsi = rsi_at(bars, index, self.period)?;

        // Buy when RSI crosses below oversold (mean reversion: expect bounce)
        if rsi < self.oversold && self.position != 1 {
            self.position = 1;
            return Some(Signal::Buy);
        }
        // Sell when RSI crosses above overbought (mean reversion: expect drop)
        if rsi > self.overbought && self.position != -1 {
            self.position = -1;
            return Some(Signal::Sell);
        }
        // Exit to flat when RSI returns to neutral zone
        if self.position == 1 && rsi > 50.0 {
            self.position = 0;
            return Some(Signal::Close);
        }
        if self.position == -1 && rsi < 50.0 {
            self.position = 0;
            return Some(Signal::Close);
        }
        None
    }

    fn name(&self) -> &str {
        "RSI Mean Rev"
    }
}

// ── Walk-Forward Analysis ─────────────────────────────────────────

/// Walk-forward analysis result.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WalkForwardResult {
    pub windows: Vec<WalkForwardWindow>,
    pub oos_sharpe: f64,        // out-of-sample Sharpe
    pub oos_profit_factor: f64, // out-of-sample PF
    pub oos_win_rate: f64,      // out-of-sample win rate
    pub robustness_score: f64,  // oos_sharpe / is_sharpe ratio (>0.5 = robust)
    pub best_params: (usize, usize), // optimal parameters from walk-forward
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WalkForwardWindow {
    pub window_idx: usize,
    pub is_start: usize,     // in-sample start bar
    pub is_end: usize,       // in-sample end bar
    pub oos_start: usize,    // out-of-sample start bar
    pub oos_end: usize,      // out-of-sample end bar
    pub best_fast: usize,
    pub best_slow: usize,
    pub is_sharpe: f64,      // in-sample Sharpe with best params
    pub oos_sharpe: f64,     // out-of-sample Sharpe with those params
    pub oos_pnl: f64,
    pub oos_trades: usize,
}

/// Run walk-forward optimization.
/// Splits bars into `num_windows` rolling windows.
/// For each window: optimize on in-sample (70%), validate on out-of-sample (30%).
pub fn walk_forward(
    bars: &[Bar],
    fast_range: std::ops::Range<usize>,
    slow_range: std::ops::Range<usize>,
    num_windows: usize,
    equity: f64,
) -> WalkForwardResult {
    let n = bars.len();
    if n < 200 || num_windows < 2 {
        return WalkForwardResult {
            windows: Vec::new(), oos_sharpe: 0.0, oos_profit_factor: 0.0,
            oos_win_rate: 0.0, robustness_score: 0.0, best_params: (10, 50),
        };
    }

    let window_size = n / num_windows;
    let is_size = (window_size as f64 * 0.7) as usize;
    let mut windows = Vec::new();
    let mut oos_trades_all: Vec<Trade> = Vec::new();

    for w in 0..num_windows {
        let start = w * window_size;
        let is_end = start + is_size;
        let oos_end = (start + window_size).min(n);
        if oos_end <= is_end { continue; }

        // Optimize on in-sample
        let is_bars = &bars[start..is_end];
        let mut best_sharpe = f64::NEG_INFINITY;
        let mut best_fast = fast_range.start;
        let mut best_slow = slow_range.start;

        let mut fast = fast_range.start;
        while fast < fast_range.end {
            let mut slow = slow_range.start;
            while slow < slow_range.end {
                if fast < slow && slow <= is_bars.len() {
                    let mut strat = SMACrossStrategy::new(fast, slow);
                    let result = run_backtest(is_bars, &mut strat, equity);
                    if result.report.sharpe_ratio > best_sharpe {
                        best_sharpe = result.report.sharpe_ratio;
                        best_fast = fast;
                        best_slow = slow;
                    }
                }
                slow += 5;
            }
            fast += 2;
        }

        // Validate on out-of-sample with best params
        let oos_bars = &bars[is_end..oos_end];
        let mut oos_strat = SMACrossStrategy::new(best_fast, best_slow);
        let oos_result = run_backtest(oos_bars, &mut oos_strat, equity);

        oos_trades_all.extend(oos_result.trades.iter().cloned());

        windows.push(WalkForwardWindow {
            window_idx: w,
            is_start: start, is_end,
            oos_start: is_end, oos_end,
            best_fast, best_slow,
            is_sharpe: best_sharpe,
            oos_sharpe: oos_result.report.sharpe_ratio,
            oos_pnl: oos_result.report.total_pnl,
            oos_trades: oos_result.trades.len(),
        });
    }

    // Aggregate OOS results
    let total_oos_trades = oos_trades_all.len();
    let oos_wins = oos_trades_all.iter().filter(|t| t.pnl > 0.0).count();
    let oos_gross_win: f64 = oos_trades_all.iter().filter(|t| t.pnl > 0.0).map(|t| t.pnl).sum();
    let oos_gross_loss: f64 = oos_trades_all.iter().filter(|t| t.pnl <= 0.0).map(|t| t.pnl.abs()).sum();

    let avg_is_sharpe = if !windows.is_empty() {
        windows.iter().map(|w| w.is_sharpe).sum::<f64>() / windows.len() as f64
    } else { 0.0 };
    let avg_oos_sharpe = if !windows.is_empty() {
        windows.iter().map(|w| w.oos_sharpe).sum::<f64>() / windows.len() as f64
    } else { 0.0 };
    let robustness = if avg_is_sharpe > 0.0 { avg_oos_sharpe / avg_is_sharpe } else { 0.0 };

    // Most common best params across windows
    let most_common_fast = windows.iter()
        .map(|w| w.best_fast)
        .max_by_key(|&f| windows.iter().filter(|w| w.best_fast == f).count())
        .unwrap_or(10);
    let most_common_slow = windows.iter()
        .map(|w| w.best_slow)
        .max_by_key(|&s| windows.iter().filter(|w| w.best_slow == s).count())
        .unwrap_or(50);

    WalkForwardResult {
        windows,
        oos_sharpe: avg_oos_sharpe,
        oos_profit_factor: if oos_gross_loss > 0.0 { oos_gross_win / oos_gross_loss } else { 0.0 },
        oos_win_rate: if total_oos_trades > 0 { oos_wins as f64 / total_oos_trades as f64 } else { 0.0 },
        robustness_score: robustness,
        best_params: (most_common_fast, most_common_slow),
    }
}

// ── Backtest Engine ─────────────────────────────────────────────────

// ── Bar-by-bar State (for visual replay) ────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BarState {
    pub bar_index: usize,
    pub timestamp: String,
    pub equity: f64,
    pub position_size: f64, // positive = long, negative = short, 0 = flat
    pub signal: Option<Signal>,
    pub trade_pnl: f64, // PnL of trade closed on this bar (0 if none)
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BarByBarResult {
    pub states: Vec<BarState>,
    pub trades: Vec<Trade>,
    pub report: TradeReport,
}

/// Run a bar-by-bar backtest that returns the full state at each bar.
/// This lets the frontend replay the backtest visually on a chart.
pub fn bar_by_bar_backtest(
    bars: &[Bar],
    strategy: &mut dyn Strategy,
    initial_equity: f64,
) -> BarByBarResult {
    let mut trades: Vec<Trade> = Vec::new();
    let mut states: Vec<BarState> = Vec::with_capacity(bars.len());
    let mut equity = initial_equity;

    let mut in_position = false;
    let mut position_side = String::new();
    let mut entry_price = 0.0;
    let mut entry_index = 0;

    for (i, bar) in bars.iter().enumerate() {
        let signal = strategy.on_bar(bar, i, bars);
        let mut trade_pnl = 0.0;

        if let Some(ref sig) = signal {
            match sig {
                Signal::Buy => {
                    if in_position && position_side == "short" {
                        let pnl = (entry_price - bar.close) * (initial_equity / entry_price);
                        let pnl_pct = (entry_price - bar.close) / entry_price * 100.0;
                        trades.push(Trade {
                            entry_index,
                            exit_index: i,
                            entry_price,
                            exit_price: bar.close,
                            side: "short".to_string(),
                            pnl,
                            pnl_pct,
                            entry_time: bars[entry_index].timestamp.clone(),
                            exit_time: bar.timestamp.clone(),
                        });
                        equity += pnl;
                        trade_pnl = pnl;
                    }
                    in_position = true;
                    position_side = "long".to_string();
                    entry_price = bar.close;
                    entry_index = i;
                }
                Signal::Sell => {
                    if in_position && position_side == "long" {
                        let pnl = (bar.close - entry_price) * (initial_equity / entry_price);
                        let pnl_pct = (bar.close - entry_price) / entry_price * 100.0;
                        trades.push(Trade {
                            entry_index,
                            exit_index: i,
                            entry_price,
                            exit_price: bar.close,
                            side: "long".to_string(),
                            pnl,
                            pnl_pct,
                            entry_time: bars[entry_index].timestamp.clone(),
                            exit_time: bar.timestamp.clone(),
                        });
                        equity += pnl;
                        trade_pnl = pnl;
                    }
                    in_position = true;
                    position_side = "short".to_string();
                    entry_price = bar.close;
                    entry_index = i;
                }
                Signal::Close => {
                    if in_position {
                        let pnl = if position_side == "long" {
                            (bar.close - entry_price) * (initial_equity / entry_price)
                        } else {
                            (entry_price - bar.close) * (initial_equity / entry_price)
                        };
                        let pnl_pct = if position_side == "long" {
                            (bar.close - entry_price) / entry_price * 100.0
                        } else {
                            (entry_price - bar.close) / entry_price * 100.0
                        };
                        trades.push(Trade {
                            entry_index,
                            exit_index: i,
                            entry_price,
                            exit_price: bar.close,
                            side: position_side.clone(),
                            pnl,
                            pnl_pct,
                            entry_time: bars[entry_index].timestamp.clone(),
                            exit_time: bar.timestamp.clone(),
                        });
                        equity += pnl;
                        trade_pnl = pnl;
                        in_position = false;
                    }
                }
            }
        }

        let position_size = if !in_position {
            0.0
        } else if position_side == "long" {
            initial_equity / entry_price
        } else {
            -(initial_equity / entry_price)
        };

        states.push(BarState {
            bar_index: i,
            timestamp: bar.timestamp.clone(),
            equity,
            position_size,
            signal,
            trade_pnl,
        });
    }

    // Close any open position at last bar
    if in_position && !bars.is_empty() {
        let last = bars.last().unwrap();
        let last_idx = bars.len() - 1;
        let pnl = if position_side == "long" {
            (last.close - entry_price) * (initial_equity / entry_price)
        } else {
            (entry_price - last.close) * (initial_equity / entry_price)
        };
        let pnl_pct = if position_side == "long" {
            (last.close - entry_price) / entry_price * 100.0
        } else {
            (entry_price - last.close) / entry_price * 100.0
        };
        trades.push(Trade {
            entry_index,
            exit_index: last_idx,
            entry_price,
            exit_price: last.close,
            side: position_side,
            pnl,
            pnl_pct,
            entry_time: bars[entry_index].timestamp.clone(),
            exit_time: last.timestamp.clone(),
        });
        equity += pnl;
        if let Some(last_state) = states.last_mut() {
            last_state.equity = equity;
            last_state.trade_pnl = pnl;
            last_state.position_size = 0.0;
        }
    }

    let report = TradeReport::from_trades(&trades, initial_equity);

    BarByBarResult {
        states,
        trades,
        report,
    }
}

// ── Optimization ────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OptimizationResult {
    pub fast_period: usize,
    pub slow_period: usize,
    pub total_trades: usize,
    pub total_pnl: f64,
    pub profit_factor: f64,
    pub sharpe_ratio: f64,
    pub win_rate: f64,
    pub max_drawdown_pct: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OptimizationReport {
    pub results: Vec<OptimizationResult>,
    pub total_combinations: usize,
}

/// Grid-search optimization over SMA cross fast/slow period combinations.
/// Returns top N results sorted by profit factor.
pub fn optimize_sma_cross(
    bars: &[Bar],
    fast_range: (usize, usize), // (min, max) inclusive
    slow_range: (usize, usize), // (min, max) inclusive
    initial_equity: f64,
    top_n: usize,
) -> OptimizationReport {
    let mut all_results: Vec<OptimizationResult> = Vec::new();
    let mut total_combinations = 0;

    for fast in fast_range.0..=fast_range.1 {
        for slow in slow_range.0..=slow_range.1 {
            if fast >= slow { continue; }
            if slow > bars.len() { continue; }
            total_combinations += 1;

            let mut strat = SMACrossStrategy::new(fast, slow);
            let result = run_backtest(bars, &mut strat, initial_equity);

            all_results.push(OptimizationResult {
                fast_period: fast,
                slow_period: slow,
                total_trades: result.report.total_trades,
                total_pnl: result.report.total_pnl,
                profit_factor: result.report.profit_factor,
                sharpe_ratio: result.report.sharpe_ratio,
                win_rate: result.report.win_rate,
                max_drawdown_pct: result.report.max_drawdown_pct,
            });
        }
    }

    // Sort by profit factor descending (infinity goes first, then highest)
    all_results.sort_by(|a, b| {
        b.profit_factor.partial_cmp(&a.profit_factor).unwrap_or(std::cmp::Ordering::Equal)
    });

    // Take top N
    all_results.truncate(top_n);

    OptimizationReport {
        results: all_results,
        total_combinations,
    }
}

// ── Original Backtest Engine ────────────────────────────────────────

pub fn run_backtest(
    bars: &[Bar],
    strategy: &mut dyn Strategy,
    initial_equity: f64,
) -> BacktestResult {
    let mut trades: Vec<Trade> = Vec::new();
    let mut equity_curve: Vec<f64> = Vec::with_capacity(bars.len());
    let mut equity = initial_equity;

    // Position state
    let mut in_position = false;
    let mut position_side = String::new(); // "long" or "short"
    let mut entry_price = 0.0;
    let mut entry_index = 0;

    for (i, bar) in bars.iter().enumerate() {
        if let Some(signal) = strategy.on_bar(bar, i, bars) {
            match signal {
                Signal::Buy => {
                    // Close short if open
                    if in_position && position_side == "short" {
                        let pnl = (entry_price - bar.close) * (initial_equity / entry_price);
                        let pnl_pct = (entry_price - bar.close) / entry_price * 100.0;
                        trades.push(Trade {
                            entry_index,
                            exit_index: i,
                            entry_price,
                            exit_price: bar.close,
                            side: "short".to_string(),
                            pnl,
                            pnl_pct,
                            entry_time: bars[entry_index].timestamp.clone(),
                            exit_time: bar.timestamp.clone(),
                        });
                        equity += pnl;
                    }
                    // Open long
                    in_position = true;
                    position_side = "long".to_string();
                    entry_price = bar.close;
                    entry_index = i;
                }
                Signal::Sell => {
                    // Close long if open
                    if in_position && position_side == "long" {
                        let pnl = (bar.close - entry_price) * (initial_equity / entry_price);
                        let pnl_pct = (bar.close - entry_price) / entry_price * 100.0;
                        trades.push(Trade {
                            entry_index,
                            exit_index: i,
                            entry_price,
                            exit_price: bar.close,
                            side: "long".to_string(),
                            pnl,
                            pnl_pct,
                            entry_time: bars[entry_index].timestamp.clone(),
                            exit_time: bar.timestamp.clone(),
                        });
                        equity += pnl;
                    }
                    // Open short
                    in_position = true;
                    position_side = "short".to_string();
                    entry_price = bar.close;
                    entry_index = i;
                }
                Signal::Close => {
                    if in_position {
                        let pnl = if position_side == "long" {
                            (bar.close - entry_price) * (initial_equity / entry_price)
                        } else {
                            (entry_price - bar.close) * (initial_equity / entry_price)
                        };
                        let pnl_pct = if position_side == "long" {
                            (bar.close - entry_price) / entry_price * 100.0
                        } else {
                            (entry_price - bar.close) / entry_price * 100.0
                        };
                        trades.push(Trade {
                            entry_index,
                            exit_index: i,
                            entry_price,
                            exit_price: bar.close,
                            side: position_side.clone(),
                            pnl,
                            pnl_pct,
                            entry_time: bars[entry_index].timestamp.clone(),
                            exit_time: bar.timestamp.clone(),
                        });
                        equity += pnl;
                        in_position = false;
                    }
                }
            }
        }
        equity_curve.push(equity);
    }

    // Close any open position at last bar
    if in_position && !bars.is_empty() {
        let last = bars.last().unwrap();
        let last_idx = bars.len() - 1;
        let pnl = if position_side == "long" {
            (last.close - entry_price) * (initial_equity / entry_price)
        } else {
            (entry_price - last.close) * (initial_equity / entry_price)
        };
        let pnl_pct = if position_side == "long" {
            (last.close - entry_price) / entry_price * 100.0
        } else {
            (entry_price - last.close) / entry_price * 100.0
        };
        trades.push(Trade {
            entry_index,
            exit_index: last_idx,
            entry_price,
            exit_price: last.close,
            side: position_side,
            pnl,
            pnl_pct,
            entry_time: bars[entry_index].timestamp.clone(),
            exit_time: last.timestamp.clone(),
        });
        equity += pnl;
        if let Some(last_eq) = equity_curve.last_mut() {
            *last_eq = equity;
        }
    }

    let report = TradeReport::from_trades(&trades, initial_equity);

    BacktestResult {
        trades,
        equity_curve,
        report,
    }
}
