//! Risk calculation and lot sizing.
//!
//! Port of all 4 order modes from TyphooN EA v1.420:
//! - Standard: Risk % of balance / SL distance
//! - Fixed: Hardcoded lot size × count
//! - Dynamic: Scale risk based on distance to min balance
//! - VaR: PercentVaR or NotionalVaR

use serde::{Deserialize, Serialize};

/// Order placement mode — matches MQL5 OrderModeEnum.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum OrderMode {
    Standard,
    Fixed,
    Dynamic,
    VaR,
}

/// VaR sub-mode — matches MQL5 VaRModeEnum.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum VaRMode {
    PercentVaR,
    NotionalVaR,
}

/// Risk calculation configuration — maps to MQL5 input parameters.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RiskConfig {
    pub order_mode: OrderMode,
    pub var_mode: VaRMode,

    // Standard mode
    pub risk_pct: f64,              // Risk % of balance (default 0.5)
    pub max_risk_pct: f64,          // Max risk cap (default 1.0)
    pub additional_risk_ratio: f64, // Reduced risk if break-even (default 0.25)

    // Fixed mode
    pub fixed_lots: f64,   // Fixed lot size (default 20)
    pub fixed_orders: u32, // Orders to place (default 2)

    // Dynamic mode
    pub min_balance: f64,   // Floor balance (default 96100)
    pub losses_to_min: u32, // Losses to reach floor (default 10)

    // VaR mode
    pub var_risk_pct: f64,     // VaR as % of equity (default 0.9)
    pub var_notional: f64,     // Fixed $ VaR target (default 9001)
    pub var_confidence: f64,   // Confidence level (default 0.95)
    pub var_timeframe: String, // Lookback timeframe (default "1Day")
    pub var_periods: u32,      // StdDev periods (default 21)

    // Account protection
    pub margin_buffer_pct: f64, // Exclude % from usable margin (default 1.0)
}

impl Default for RiskConfig {
    fn default() -> Self {
        Self {
            order_mode: OrderMode::VaR,
            var_mode: VaRMode::PercentVaR,
            risk_pct: 0.5,
            max_risk_pct: 1.0,
            additional_risk_ratio: 0.25,
            fixed_lots: 20.0,
            fixed_orders: 2,
            min_balance: 96_100.0,
            losses_to_min: 10,
            var_risk_pct: 0.9,
            var_notional: 9_001.0,
            var_confidence: 0.95,
            var_timeframe: "1Day".to_string(),
            var_periods: 21,
            margin_buffer_pct: 1.0,
        }
    }
}

/// Symbol specifications needed for lot sizing.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SymbolSpec {
    pub symbol: String,
    pub tick_size: f64,
    pub tick_value: f64,
    pub volume_min: f64,
    pub volume_max: f64,
    pub volume_step: f64,
    pub contract_size: f64,
    pub margin_rate: f64, // As fraction (0.20 for x5)
}

/// Calculate lot size from risk amount and SL distance.
///
/// Port of RiskCalc.mqh RiskLots():
///   riskAmount / (ticks × tickValue)
///   where ticks = slDistance / tickSize
pub fn risk_lots(spec: &SymbolSpec, risk_amount: f64, sl_distance: f64) -> f64 {
    if sl_distance <= 0.0 || spec.tick_size <= 0.0 || spec.tick_value <= 0.0 {
        return 0.0;
    }
    let ticks = sl_distance / spec.tick_size;
    let lots = risk_amount / (ticks * spec.tick_value);
    normalize_lots(lots, spec)
}

/// Calculate lot size using Standard risk mode.
///
/// risk_money = balance × (risk_pct / 100)
/// If break-even exists, reduce by additional_risk_ratio.
pub fn standard_lots(
    config: &RiskConfig,
    spec: &SymbolSpec,
    balance: f64,
    sl_distance: f64,
    has_break_even: bool,
) -> f64 {
    let risk_pct = if has_break_even {
        (config.risk_pct * config.additional_risk_ratio).min(config.max_risk_pct)
    } else {
        config.risk_pct.min(config.max_risk_pct)
    };
    let risk_money = balance * (risk_pct / 100.0);
    risk_lots(spec, risk_money, sl_distance)
}

/// Calculate lot size using Dynamic risk mode.
///
/// risk_money = (balance - min_balance) / losses_to_min
/// Stops trading if balance <= min_balance.
pub fn dynamic_lots(
    config: &RiskConfig,
    spec: &SymbolSpec,
    balance: f64,
    sl_distance: f64,
    has_break_even: bool,
) -> f64 {
    if balance <= config.min_balance || config.losses_to_min == 0 {
        return 0.0;
    }
    let risk_money = if has_break_even {
        (balance - config.min_balance)
            / (config.losses_to_min as f64 / config.additional_risk_ratio)
    } else {
        (balance - config.min_balance) / config.losses_to_min as f64
    };
    risk_lots(spec, risk_money, sl_distance)
}

/// Calculate lot size using VaR mode.
///
/// PercentVaR: lots to cap loss at var_risk_pct % of equity.
/// NotionalVaR: lots for fixed $ VaR target.
pub fn var_lots(config: &RiskConfig, spec: &SymbolSpec, equity: f64, var_per_lot: f64) -> f64 {
    if var_per_lot <= 0.0 {
        return 0.0;
    }
    let lots = match config.var_mode {
        VaRMode::PercentVaR => {
            if equity <= 0.0 {
                return 0.0;
            }
            let max_var = (config.var_risk_pct / 100.0) * equity;
            max_var / var_per_lot
        }
        VaRMode::NotionalVaR => config.var_notional / var_per_lot,
    };
    normalize_lots(lots, spec)
}

/// Calculate lot size based on current config and mode.
pub fn calculate_lots(
    config: &RiskConfig,
    spec: &SymbolSpec,
    balance: f64,
    equity: f64,
    sl_distance: f64,
    has_break_even: bool,
    var_per_lot: f64,
) -> (f64, u32) {
    match config.order_mode {
        OrderMode::Standard => (
            standard_lots(config, spec, balance, sl_distance, has_break_even),
            1,
        ),
        OrderMode::Fixed => (config.fixed_lots, config.fixed_orders),
        OrderMode::Dynamic => (
            dynamic_lots(config, spec, balance, sl_distance, has_break_even),
            1,
        ),
        OrderMode::VaR => (var_lots(config, spec, equity, var_per_lot), 1),
    }
}

/// Normalize lots to symbol constraints (min, max, step).
fn normalize_lots(lots: f64, spec: &SymbolSpec) -> f64 {
    if lots < spec.volume_min {
        return 0.0;
    }
    let lots = lots.min(spec.volume_max);
    // Round to volume_step
    if spec.volume_step > 0.0 {
        (lots / spec.volume_step).floor() * spec.volume_step
    } else {
        lots
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_spec() -> SymbolSpec {
        SymbolSpec {
            symbol: "TEST".to_string(),
            tick_size: 0.01,
            tick_value: 1.0,
            volume_min: 0.01,
            volume_max: 100.0,
            volume_step: 0.01,
            contract_size: 1.0,
            margin_rate: 1.0,
        }
    }

    #[test]
    fn test_risk_lots() {
        let spec = test_spec();
        // $100 risk, 10 point SL = 1000 ticks, $1/tick = 0.10 lots
        let lots = risk_lots(&spec, 100.0, 10.0);
        assert!((lots - 0.10).abs() < 0.001);
    }

    #[test]
    fn test_standard_lots() {
        let config = RiskConfig::default();
        let spec = test_spec();
        // $100K balance, 0.5% risk = $500, 10 point SL
        let lots = standard_lots(&config, &spec, 100_000.0, 10.0, false);
        assert!((lots - 0.50).abs() < 0.01);
    }

    #[test]
    fn test_var_lots_percent() {
        let config = RiskConfig {
            var_risk_pct: 1.0,
            ..Default::default()
        };
        let spec = test_spec();
        // $100K equity, 1% VaR = $1000 max, $500 VaR/lot = 2 lots
        let lots = var_lots(&config, &spec, 100_000.0, 500.0);
        assert!((lots - 2.0).abs() < 0.01);
    }

    #[test]
    fn test_risk_lots_zero_sl() {
        let spec = test_spec();
        let lots = risk_lots(&spec, 100.0, 0.0);
        assert_eq!(lots, 0.0, "Zero SL should produce zero lots");
    }

    #[test]
    fn test_risk_lots_zero_risk() {
        let spec = test_spec();
        let lots = risk_lots(&spec, 0.0, 10.0);
        assert_eq!(lots, 0.0, "Zero risk should produce zero lots");
    }

    #[test]
    fn test_standard_lots_zero_equity() {
        let config = RiskConfig::default();
        let spec = test_spec();
        let lots = standard_lots(&config, &spec, 0.0, 10.0, false);
        assert_eq!(lots, 0.0, "Zero equity should produce zero lots");
    }

    #[test]
    fn test_var_lots_zero_var_per_lot() {
        let config = RiskConfig::default();
        let spec = test_spec();
        let lots = var_lots(&config, &spec, 100_000.0, 0.0);
        assert_eq!(lots, 0.0, "Zero VaR/lot should produce zero lots");
    }

    #[test]
    fn test_normalize_lots_rounding() {
        let spec = test_spec();
        // 0.123 lots with step 0.01 → 0.12
        let normalized = normalize_lots(0.123, &spec);
        assert!(
            (normalized - 0.12).abs() < 0.001,
            "Should round down to lot step: {normalized}"
        );
    }
}
