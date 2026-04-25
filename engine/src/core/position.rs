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
    pub side: String, // "long" or "short"
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

#[cfg(test)]
mod tests {
    use super::*;

    // ── Direction ───────────────────────────────────────────────────

    #[test]
    fn direction_opposite() {
        assert_eq!(Direction::Long.opposite(), Direction::Short);
        assert_eq!(Direction::Short.opposite(), Direction::Long);
    }

    #[test]
    fn direction_side_str() {
        assert_eq!(Direction::Long.side_str(), "long");
        assert_eq!(Direction::Short.side_str(), "short");
    }

    // ── BrokerPosition ─────────────────────────────────────────────

    #[test]
    fn is_break_even_true() {
        let pos = BrokerPosition {
            id: "1".into(),
            symbol: "AAPL".into(),
            qty: 10.0,
            side: "long".into(),
            avg_entry: 150.0,
            current_price: 155.0,
            unrealized_pl: 50.0,
            market_value: 1550.0,
            sl: Some(150.0),
            tp: Some(160.0),
        };
        assert!(pos.is_break_even(0.01)); // SL == entry within 0.005 tolerance
    }

    #[test]
    fn is_break_even_false_no_sl() {
        let pos = BrokerPosition {
            id: "1".into(),
            symbol: "AAPL".into(),
            qty: 10.0,
            side: "long".into(),
            avg_entry: 150.0,
            current_price: 155.0,
            unrealized_pl: 50.0,
            market_value: 1550.0,
            sl: None,
            tp: Some(160.0),
        };
        assert!(!pos.is_break_even(0.01));
    }

    #[test]
    fn is_break_even_false_sl_far() {
        let pos = BrokerPosition {
            id: "1".into(),
            symbol: "AAPL".into(),
            qty: 10.0,
            side: "long".into(),
            avg_entry: 150.0,
            current_price: 155.0,
            unrealized_pl: 50.0,
            market_value: 1550.0,
            sl: Some(145.0),
            tp: Some(160.0),
        };
        assert!(!pos.is_break_even(0.01));
    }

    #[test]
    fn sl_pl_long_position() {
        let pos = BrokerPosition {
            id: "1".into(),
            symbol: "AAPL".into(),
            qty: 100.0,
            side: "long".into(),
            avg_entry: 150.0,
            current_price: 155.0,
            unrealized_pl: 500.0,
            market_value: 15500.0,
            sl: Some(145.0),
            tp: Some(160.0),
        };
        // SL loss = (145 - 150) * 100 = -500
        assert!((pos.sl_pl() - (-500.0)).abs() < 1e-10);
    }

    #[test]
    fn sl_pl_short_position() {
        let pos = BrokerPosition {
            id: "1".into(),
            symbol: "AAPL".into(),
            qty: 100.0,
            side: "short".into(),
            avg_entry: 150.0,
            current_price: 145.0,
            unrealized_pl: 500.0,
            market_value: 14500.0,
            sl: Some(155.0),
            tp: Some(140.0),
        };
        // SL loss = (150 - 155) * 100 = -500
        assert!((pos.sl_pl() - (-500.0)).abs() < 1e-10);
    }

    #[test]
    fn sl_pl_no_sl() {
        let pos = BrokerPosition {
            id: "1".into(),
            symbol: "AAPL".into(),
            qty: 100.0,
            side: "long".into(),
            avg_entry: 150.0,
            current_price: 155.0,
            unrealized_pl: 500.0,
            market_value: 15500.0,
            sl: None,
            tp: Some(160.0),
        };
        assert_eq!(pos.sl_pl(), 0.0);
    }

    #[test]
    fn tp_pl_long_position() {
        let pos = BrokerPosition {
            id: "1".into(),
            symbol: "AAPL".into(),
            qty: 100.0,
            side: "long".into(),
            avg_entry: 150.0,
            current_price: 155.0,
            unrealized_pl: 500.0,
            market_value: 15500.0,
            sl: Some(145.0),
            tp: Some(160.0),
        };
        // TP profit = (160 - 150) * 100 = 1000
        assert!((pos.tp_pl() - 1000.0).abs() < 1e-10);
    }

    #[test]
    fn tp_pl_short_position() {
        let pos = BrokerPosition {
            id: "1".into(),
            symbol: "AAPL".into(),
            qty: 100.0,
            side: "short".into(),
            avg_entry: 150.0,
            current_price: 145.0,
            unrealized_pl: 500.0,
            market_value: 14500.0,
            sl: Some(155.0),
            tp: Some(140.0),
        };
        // TP profit = (150 - 140) * 100 = 1000
        assert!((pos.tp_pl() - 1000.0).abs() < 1e-10);
    }

    // ── HedgedPosition ─────────────────────────────────────────────

    #[test]
    fn hedged_position_new() {
        let hp = HedgedPosition::new("SOLUSD".into(), Direction::Short);
        assert_eq!(hp.symbol, "SOLUSD");
        assert_eq!(hp.bias_direction, Direction::Short);
        assert_eq!(hp.bias_qty, 0.0);
        assert_eq!(hp.hedge_qty, 0.0);
    }

    #[test]
    fn hedged_position_net_and_gross() {
        let mut hp = HedgedPosition::new("SOLUSD".into(), Direction::Short);
        hp.bias_qty = 100.0;
        hp.hedge_qty = 40.0;
        assert!((hp.net_qty() - 60.0).abs() < 1e-10);
        assert!((hp.gross_qty() - 140.0).abs() < 1e-10);
        assert!(!hp.is_pure_bias());
    }

    #[test]
    fn hedged_position_pure_bias() {
        let mut hp = HedgedPosition::new("SOLUSD".into(), Direction::Short);
        hp.bias_qty = 100.0;
        hp.hedge_qty = 0.0;
        assert!(hp.is_pure_bias());
    }

    #[test]
    fn hedged_position_sides() {
        let hp = HedgedPosition::new("SOLUSD".into(), Direction::Short);
        assert_eq!(hp.bias_side(), "short");
        assert_eq!(hp.hedge_side(), "long");

        let hp2 = HedgedPosition::new("DOGE".into(), Direction::Long);
        assert_eq!(hp2.bias_side(), "long");
        assert_eq!(hp2.hedge_side(), "short");
    }

    // ── BreakEvenState ─────────────────────────────────────────────

    #[test]
    fn break_even_state_any() {
        let mut be = BreakEvenState::default();
        assert!(!be.any());
        be.long_break_even = true;
        assert!(be.any());
    }

    // ── PositionSummary risk_reward_ratio ───────────────────────────

    #[test]
    fn risk_reward_ratio_normal() {
        let ps = PositionSummary {
            symbol: "AAPL".into(),
            long_lots: 100.0,
            short_lots: 0.0,
            net_lots: 100.0,
            gross_lots: 100.0,
            total_unrealized_pl: 500.0,
            total_sl_pl: -500.0,
            total_tp_pl: 1000.0,
            break_even: BreakEvenState::default(),
        };
        assert!((ps.risk_reward_ratio() - 2.0).abs() < 1e-10);
    }

    #[test]
    fn risk_reward_ratio_zero_sl() {
        let ps = PositionSummary {
            symbol: "AAPL".into(),
            long_lots: 100.0,
            short_lots: 0.0,
            net_lots: 100.0,
            gross_lots: 100.0,
            total_unrealized_pl: 500.0,
            total_sl_pl: 0.0,
            total_tp_pl: 1000.0,
            break_even: BreakEvenState::default(),
        };
        assert_eq!(ps.risk_reward_ratio(), 0.0); // avoid divide by zero
    }
}
