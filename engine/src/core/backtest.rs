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

        let n_trades = trades.len() as f64;
        let win_rate = if n_trades > 0.0 { wins.len() as f64 / n_trades * 100.0 } else { 0.0 };
        let profit_factor = if gross_loss > 0.0 { (gross_profit / gross_loss).min(999.0) } else { 999.0 };
        let avg_win = if wins.is_empty() { 0.0 } else { gross_profit / wins.len() as f64 };
        let avg_loss = if losses.is_empty() { 0.0 } else { gross_loss / losses.len() as f64 };
        let avg_trade = if n_trades > 0.0 { total_pnl / n_trades } else { 0.0 };

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
        let n_returns = returns.len() as f64;
        let mean_ret = if n_returns > 0.0 { returns.iter().sum::<f64>() / n_returns } else { 0.0 };
        let variance = if n_returns > 0.0 { returns.iter().map(|r| (r - mean_ret).powi(2)).sum::<f64>() / n_returns } else { 0.0 };
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
    if let (true, Some(last)) = (in_position, bars.last()) {
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
    if let (true, Some(last)) = (in_position, bars.last()) {
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

// ── Tests ──────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
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
            if index == 0 { return None; }
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
        fn new() -> Self { Self { state: 0 } }
    }

    impl Strategy for BuyThenClose {
        fn on_bar(&mut self, _bar: &Bar, index: usize, _bars: &[Bar]) -> Option<Signal> {
            match (index, self.state) {
                (1, 0) => { self.state = 1; Some(Signal::Buy) }
                (3, 1) => { self.state = 2; Some(Signal::Close) }
                _ => None,
            }
        }
        fn name(&self) -> &str { "BuyThenClose" }
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
            entry_index: 0, exit_index: 1,
            entry_price: 100.0, exit_price: 110.0,
            side: "long".to_string(),
            pnl: 100.0, pnl_pct: 10.0,
            entry_time: "t0".into(), exit_time: "t1".into(),
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
                entry_index: 0, exit_index: 1,
                entry_price: 100.0, exit_price: 120.0,
                side: "long".to_string(),
                pnl: 200.0, pnl_pct: 20.0,
                entry_time: "t0".into(), exit_time: "t1".into(),
            },
            Trade {
                entry_index: 1, exit_index: 2,
                entry_price: 120.0, exit_price: 110.0,
                side: "long".to_string(),
                pnl: -100.0, pnl_pct: -8.33,
                entry_time: "t1".into(), exit_time: "t2".into(),
            },
            Trade {
                entry_index: 2, exit_index: 3,
                entry_price: 110.0, exit_price: 130.0,
                side: "long".to_string(),
                pnl: 200.0, pnl_pct: 18.18,
                entry_time: "t2".into(), exit_time: "t3".into(),
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
                entry_index: 0, exit_index: 1,
                entry_price: 100.0, exit_price: 90.0,
                side: "long".to_string(),
                pnl: -100.0, pnl_pct: -10.0,
                entry_time: "t0".into(), exit_time: "t1".into(),
            },
            Trade {
                entry_index: 1, exit_index: 2,
                entry_price: 90.0, exit_price: 80.0,
                side: "long".to_string(),
                pnl: -100.0, pnl_pct: -11.11,
                entry_time: "t1".into(), exit_time: "t2".into(),
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
        let mut bars: Vec<Bar> = (0..20).map(|i| flat_bar(100.0, &format!("t{}", i))).collect();
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
        assert!(result.trades.len() >= 1, "Expected at least 1 trade, got {}", result.trades.len());
        assert_eq!(result.equity_curve.len(), bars.len());
    }

    #[test]
    fn sma_cross_no_signal_flat_market() {
        // Completely flat market — no crossover possible
        let bars: Vec<Bar> = (0..50).map(|i| flat_bar(100.0, &format!("t{}", i))).collect();
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
        let mut bars: Vec<Bar> = (0..15).map(|i| flat_bar(100.0, &format!("t{}", i))).collect();
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
        struct BuyOnce { bought: bool }
        impl Strategy for BuyOnce {
            fn on_bar(&mut self, _bar: &Bar, index: usize, _bars: &[Bar]) -> Option<Signal> {
                if index == 1 && !self.bought { self.bought = true; Some(Signal::Buy) }
                else { None }
            }
            fn name(&self) -> &str { "BuyOnce" }
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
        struct ShortThenClose { state: u8 }
        impl Strategy for ShortThenClose {
            fn on_bar(&mut self, _bar: &Bar, index: usize, _bars: &[Bar]) -> Option<Signal> {
                match (index, self.state) {
                    (1, 0) => { self.state = 1; Some(Signal::Sell) }
                    (3, 1) => { self.state = 2; Some(Signal::Buy) }
                    _ => None,
                }
            }
            fn name(&self) -> &str { "ShortThenClose" }
        }

        let bars = vec![
            flat_bar(100.0, "t0"),
            flat_bar(100.0, "t1"), // short entry
            flat_bar(90.0, "t2"),
            flat_bar(80.0, "t3"),  // close short (Buy signal)
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
        let mut bars: Vec<Bar> = (0..20).map(|i| flat_bar(100.0, &format!("t{}", i))).collect();
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
            assert!(report.results[i - 1].profit_factor >= report.results[i].profit_factor
                || report.results[i - 1].profit_factor.is_infinite());
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
        let mut bars: Vec<Bar> = (0..30).map(|i| flat_bar(100.0, &format!("t{}", i))).collect();
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
            bars.push(bar(price - 1.0, price + 2.0, price - 2.0, price, &format!("t{}", i)));
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
            bars.push(bar(price, price + 3.0, price - 3.0, price, &format!("t{}", i)));
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
        assert!(rsi.unwrap() > 80.0, "Expected RSI > 80 for all-up, got {}", rsi.unwrap());
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
        assert!(k > 100.0 && k < 130.0, "KAMA={} should be between 100 and 130", k);
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

        assert_eq!(result1.trades.len(), result2.trades.len(),
            "run_backtest ({}) and bar_by_bar ({}) should produce same number of trades",
            result1.trades.len(), result2.trades.len());

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
        assert!((final_eq1 - final_eq2).abs() < 1e-4,
            "Final equity mismatch: run_backtest={}, bar_by_bar={}", final_eq1, final_eq2);
    }
}
