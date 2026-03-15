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

// ── Backtest Engine ─────────────────────────────────────────────────

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
