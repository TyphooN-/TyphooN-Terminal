//! Position tracking and classification.
//!
//! Tracks hedge vs bias, net/gross exposure, break-even detection,
//! SL/TP management, and P/L calculation.

use serde::{Deserialize, Serialize};

/// Bias direction for martingale.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum Direction {
    Long,
    Short,
}

impl Direction {
    pub fn opposite(&self) -> Self {
        match self {
            Direction::Long => Direction::Short,
            Direction::Short => Direction::Long,
        }
    }

    pub fn side_str(&self) -> &'static str {
        match self {
            Direction::Long => "long",
            Direction::Short => "short",
        }
    }
}

/// Individual position from the broker.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BrokerPosition {
    pub id: String,
    pub symbol: String,
    pub qty: f64,
    pub side: String,       // "long" or "short"
    pub avg_entry: f64,
    pub current_price: f64,
    pub unrealized_pl: f64,
    pub market_value: f64,
    pub sl: Option<f64>,
    pub tp: Option<f64>,
}

impl BrokerPosition {
    /// Check if position is at break-even (SL == entry within tick tolerance).
    pub fn is_break_even(&self, tick_size: f64) -> bool {
        if let Some(sl) = self.sl {
            (sl - self.avg_entry).abs() < tick_size * 0.5
        } else {
            false
        }
    }

    /// Calculate P/L if SL is hit.
    pub fn sl_pl(&self) -> f64 {
        if let Some(sl) = self.sl {
            let distance = if self.side == "long" {
                sl - self.avg_entry
            } else {
                self.avg_entry - sl
            };
            distance * self.qty
        } else {
            0.0
        }
    }

    /// Calculate P/L if TP is hit.
    pub fn tp_pl(&self) -> f64 {
        if let Some(tp) = self.tp {
            let distance = if self.side == "long" {
                tp - self.avg_entry
            } else {
                self.avg_entry - tp
            };
            distance * self.qty
        } else {
            0.0
        }
    }
}

/// Hedged martingale position tracking for a single symbol.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HedgedPosition {
    pub symbol: String,
    pub bias_direction: Direction,
    pub bias_qty: f64,
    pub hedge_qty: f64,
    pub bias_avg_entry: f64,
    pub hedge_avg_entry: f64,
    pub trim_closes: u64,
    pub protect_closes: u64,
}

impl HedgedPosition {
    pub fn new(symbol: String, direction: Direction) -> Self {
        Self {
            symbol,
            bias_direction: direction,
            bias_qty: 0.0,
            hedge_qty: 0.0,
            bias_avg_entry: 0.0,
            hedge_avg_entry: 0.0,
            trim_closes: 0,
            protect_closes: 0,
        }
    }

    /// Net exposure in bias direction.
    pub fn net_qty(&self) -> f64 {
        self.bias_qty - self.hedge_qty
    }

    /// Total lots both sides.
    pub fn gross_qty(&self) -> f64 {
        self.bias_qty + self.hedge_qty
    }

    /// All hedge lots consumed.
    pub fn is_pure_bias(&self) -> bool {
        self.hedge_qty <= 0.0
    }

    /// Side string for hedge positions.
    pub fn hedge_side(&self) -> &'static str {
        self.bias_direction.opposite().side_str()
    }

    /// Side string for bias positions.
    pub fn bias_side(&self) -> &'static str {
        self.bias_direction.side_str()
    }
}

/// Break-even state for a symbol — per-direction flags.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct BreakEvenState {
    pub long_break_even: bool,
    pub short_break_even: bool,
}

impl BreakEvenState {
    pub fn any(&self) -> bool {
        self.long_break_even || self.short_break_even
    }
}

/// Aggregate position info for a symbol.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PositionSummary {
    pub symbol: String,
    pub long_lots: f64,
    pub short_lots: f64,
    pub net_lots: f64,
    pub gross_lots: f64,
    pub total_unrealized_pl: f64,
    pub total_sl_pl: f64,
    pub total_tp_pl: f64,
    pub break_even: BreakEvenState,
}

impl PositionSummary {
    pub fn risk_reward_ratio(&self) -> f64 {
        if self.total_sl_pl.abs() < 1e-10 {
            return 0.0;
        }
        self.total_tp_pl / self.total_sl_pl.abs()
    }
}
