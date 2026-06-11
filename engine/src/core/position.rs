//! Position tracking and classification.
//!
//! Tracks hedge vs bias, net/gross exposure, break-even detection,
//! SL/TP management, and P/L calculation.

use serde::{Deserialize, Serialize};

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
