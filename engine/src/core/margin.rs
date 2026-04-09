//! Margin calculation utilities.
//!
//! Port of forward-looking TRIM margin math from MQL5 TyphooN EA v1.420.
//! Instrument-agnostic — works for stocks, crypto, or any asset class.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MarginState {
    pub equity: f64,
    pub margin_used: f64,
    pub margin_level_pct: f64,
}

impl MarginState {
    pub fn new(equity: f64, margin: f64) -> Self {
        let ml = if margin > 0.01 {
            equity / margin * 100.0
        } else {
            999.0
        };
        Self {
            equity,
            margin_used: margin,
            margin_level_pct: ml,
        }
    }
}

/// Forward-looking TRIM: compute how many hedge lots can be closed before ML hits threshold.
///
/// Each hedge close increases net exposure → increases margin → lowers ML.
/// Computes the exact count that brings ML to the threshold, never below.
///
/// Port of:
///   maxMargin = equity / (threshold / 100)
///   availableRoom = maxMargin - currentMargin
///   maxSafeLots = floor(availableRoom / marginPerLot)
pub fn max_safe_lots(
    equity: f64,
    current_margin: f64,
    margin_per_lot: f64,
    trim_threshold_pct: f64,
) -> u64 {
    if margin_per_lot <= 0.0 || trim_threshold_pct <= 0.0 {
        return 0;
    }
    let max_margin = equity / (trim_threshold_pct / 100.0);
    let available_room = max_margin - current_margin;
    if available_room <= 0.0 {
        return 0;
    }
    (available_room / margin_per_lot) as u64
}

/// Spread tolerance = equity / gross lots.
///
/// The REAL survival metric, not margin level.
/// Broker charges margin on net, but spreads hit gross.
pub fn spread_tolerance(equity: f64, gross_lots: f64) -> f64 {
    if gross_lots <= 0.0 {
        return f64::INFINITY;
    }
    equity / gross_lots
}

/// Dynamic PROTECT sizing: urgency scales with how far below threshold ML has dropped.
///
/// Returns a fraction [0.01, 1.0] representing how aggressively to close.
/// Port of: urgency = max(1 - ML/threshold, 0.01)
pub fn protect_urgency(margin_level_pct: f64, protect_threshold_pct: f64) -> f64 {
    if protect_threshold_pct <= 0.0 {
        return 0.01;
    }
    (1.0 - (margin_level_pct / protect_threshold_pct)).max(0.01)
}

/// How many lots to close per PROTECT fire.
///
/// Port of: ceil(totalHedgeLots * urgency)
pub fn protect_lot_count(total_hedge_lots: f64, urgency: f64, min_lot: f64) -> f64 {
    (total_hedge_lots * urgency).ceil().max(min_lot)
}

/// Calculate margin level percentage.
/// Port of: (equity / margin) * 100, returns 999 if no margin.
pub fn margin_level_pct(equity: f64, margin: f64) -> f64 {
    if margin <= 0.01 {
        999.0
    } else {
        equity / margin * 100.0
    }
}

/// Calculate usable margin with buffer.
/// Port of: (balance * (1 - buffer/100)) - currentMargin
pub fn usable_margin(balance: f64, current_margin: f64, buffer_pct: f64) -> f64 {
    let budget = balance * (1.0 - buffer_pct / 100.0);
    (budget - current_margin).max(0.0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_max_safe_lots_basic() {
        // $100K equity, $0 margin, $600/lot, 65% threshold
        assert_eq!(max_safe_lots(100_000.0, 0.0, 600.0, 65.0), 256);
    }

    #[test]
    fn test_max_safe_lots_leveraged() {
        // x5 leverage, TRIM at 80%
        assert_eq!(max_safe_lots(100_000.0, 0.0, 600.0, 80.0), 208);
    }

    #[test]
    fn test_max_safe_lots_no_room() {
        assert_eq!(max_safe_lots(50_000.0, 100_000.0, 600.0, 65.0), 0);
    }

    #[test]
    fn test_spread_tolerance() {
        assert!((spread_tolerance(100_000.0, 50_000.0) - 2.0).abs() < 0.001);
        assert!(spread_tolerance(100_000.0, 0.0).is_infinite());
    }

    #[test]
    fn test_protect_urgency() {
        let u = protect_urgency(40.0, 56.0);
        assert!((u - 0.2857).abs() < 0.001);
    }

    #[test]
    fn test_usable_margin() {
        // $100K balance, $50K margin, 1% buffer
        let usable = usable_margin(100_000.0, 50_000.0, 1.0);
        assert!((usable - 49_000.0).abs() < 1.0);
    }

    #[test]
    fn test_usable_margin_zero_balance() {
        let usable = usable_margin(0.0, 0.0, 1.0);
        assert!(usable <= 0.0);
    }

    #[test]
    fn test_spread_tolerance_equal() {
        let st = spread_tolerance(50_000.0, 50_000.0);
        assert!((st - 1.0).abs() < 0.001);
    }

    #[test]
    fn test_protect_urgency_at_threshold() {
        let u = protect_urgency(56.0, 56.0);
        // At threshold: (1.0 - 1.0).max(0.01) = 0.01 (minimum urgency)
        assert!((u - 0.01).abs() < 0.001, "At threshold, urgency should be minimum 0.01, got {u}");
    }

    #[test]
    fn test_protect_urgency_below_threshold() {
        let u = protect_urgency(30.0, 56.0);
        assert!(u > 0.0);
    }
}
