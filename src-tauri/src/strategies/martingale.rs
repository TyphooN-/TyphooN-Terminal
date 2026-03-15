//! Hedged Martingale Strategy Engine.
//!
//! Port of TyphooN EA v1.420 martingale state machine:
//! - MG_OFF → MG_LONG/SHORT → MG_UNWIND → MG_OFF
//! - Forward-looking TRIM (close hedge to build directional exposure)
//! - Dynamic PROTECT (balanced close when margin critical)
//! - Hard floor (broker handles it)
//! - Bias protection (never close directional positions)
//! - Open MG (one-click full hedge position setup)
//! - Equity TP (close all at profit target)
//! - Unwind (close worst P/L first)

use serde::{Deserialize, Serialize};

use crate::core::margin::{max_safe_lots, protect_urgency, protect_lot_count};
use crate::core::position::HedgedPosition;

/// Martingale mode — matches MQL5 MartingaleState enum.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum MartingaleMode {
    Off,
    Long,
    Short,
    Unwind,
}

impl MartingaleMode {
    /// Cycle to next mode (button state machine).
    pub fn next(self) -> Self {
        match self {
            Self::Off => Self::Long,
            Self::Long => Self::Short,
            Self::Short => Self::Unwind,
            Self::Unwind => Self::Off,
        }
    }

    pub fn label(&self) -> &'static str {
        match self {
            Self::Off => "MG: OFF",
            Self::Long => "MG: LONG",
            Self::Short => "MG: SHORT",
            Self::Unwind => "MG: UNWIND",
        }
    }
}

/// What the engine decided to do on this tick.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TickDecision {
    pub action: Action,
    pub close_side: Option<String>,
    pub close_qty: f64,
    pub reason: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum Action {
    Trim,
    Protect,
    DeadZone,
    HardFloor,
    PureBias,
    EquityTP,
    Unwind,
    Idle,
}

/// Martingale configuration — maps to MQL5 input parameters.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MartingaleConfig {
    pub enabled: bool,
    pub trim_pct: f64,              // TRIM threshold (default 65, 80 for CFD)
    pub protect_pct: f64,           // PROTECT threshold (default 56)
    pub hard_floor_pct: f64,        // Hard floor (default 10)
    pub spread_tolerance: f64,      // $ per lot for Open MG sizing
    pub equity_tp: f64,             // $ profit target (0 = disabled)
}

impl Default for MartingaleConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            trim_pct: 65.0,
            protect_pct: 56.0,
            hard_floor_pct: 10.0,
            spread_tolerance: 2.0,
            equity_tp: 0.0,
        }
    }
}

/// Martingale engine state — persisted across ticks.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MartingaleState {
    pub mode: MartingaleMode,
    pub config: MartingaleConfig,
    pub hedge_closes: u64,
    pub bias_closes: u64,
    pub protect_fire_count: u64,
    pub protect_active: bool,
    // Baseline tracking (for delta logging)
    pub init_equity: f64,
    pub init_balance: f64,
    pub init_margin_pct: f64,
    pub init_hedge_lots: f64,
    pub init_bias_lots: f64,
    pub init_net_lots: f64,
}

impl MartingaleState {
    pub fn new(config: MartingaleConfig) -> Self {
        Self {
            mode: MartingaleMode::Off,
            config,
            hedge_closes: 0,
            bias_closes: 0,
            protect_fire_count: 0,
            protect_active: false,
            init_equity: 0.0,
            init_balance: 0.0,
            init_margin_pct: 0.0,
            init_hedge_lots: 0.0,
            init_bias_lots: 0.0,
            init_net_lots: 0.0,
        }
    }

    /// Capture baseline when MG mode is activated.
    pub fn capture_baseline(
        &mut self,
        equity: f64,
        balance: f64,
        margin_pct: f64,
        position: &HedgedPosition,
    ) {
        self.init_equity = equity;
        self.init_balance = balance;
        self.init_margin_pct = margin_pct;
        self.init_hedge_lots = position.hedge_qty;
        self.init_bias_lots = position.bias_qty;
        self.init_net_lots = position.net_qty();
    }

    /// Evaluate current state and return the action to take.
    pub fn decide(
        &self,
        equity: f64,
        margin_used: f64,
        margin_per_unit: f64,
        position: &HedgedPosition,
        mg_pl: f64,
    ) -> TickDecision {
        if self.mode == MartingaleMode::Off {
            return TickDecision {
                action: Action::Idle,
                close_side: None,
                close_qty: 0.0,
                reason: "Martingale disabled".to_string(),
            };
        }

        if self.mode == MartingaleMode::Unwind {
            return TickDecision {
                action: Action::Unwind,
                close_side: None,
                close_qty: 0.0,
                reason: "Unwind mode — close worst P/L position".to_string(),
            };
        }

        // Equity TP check
        if self.config.equity_tp > 0.0 && mg_pl >= self.config.equity_tp {
            return TickDecision {
                action: Action::EquityTP,
                close_side: None,
                close_qty: 0.0,
                reason: format!(
                    "MG Equity TP: P/L ${:.2} >= target ${:.2}",
                    mg_pl, self.config.equity_tp
                ),
            };
        }

        if position.is_pure_bias() {
            return TickDecision {
                action: Action::PureBias,
                close_side: None,
                close_qty: 0.0,
                reason: "All hedge lots consumed — pure bias".to_string(),
            };
        }

        let ml = if margin_used > 0.01 {
            equity / margin_used * 100.0
        } else {
            999.0
        };

        // Hard floor
        if ml <= self.config.hard_floor_pct {
            return TickDecision {
                action: Action::HardFloor,
                close_side: None,
                close_qty: 0.0,
                reason: format!(
                    "ML {:.1}% <= hard floor {:.1}% — broker handles it",
                    ml, self.config.hard_floor_pct
                ),
            };
        }

        // PROTECT zone
        if ml < self.config.protect_pct {
            let urgency = protect_urgency(ml, self.config.protect_pct);
            let qty = protect_lot_count(position.hedge_qty, urgency, 1.0);
            return TickDecision {
                action: Action::Protect,
                close_side: Some("both".to_string()),
                close_qty: qty,
                reason: format!(
                    "ML {:.1}% < PROTECT {:.1}% — urgency {:.2}, close {:.0} per side",
                    ml, self.config.protect_pct, urgency, qty
                ),
            };
        }

        // Dead zone
        if ml <= self.config.trim_pct {
            return TickDecision {
                action: Action::DeadZone,
                close_side: None,
                close_qty: 0.0,
                reason: format!(
                    "ML {:.1}% in dead zone ({:.1}-{:.1}%)",
                    ml, self.config.protect_pct, self.config.trim_pct
                ),
            };
        }

        // TRIM zone
        let safe_lots = max_safe_lots(equity, margin_used, margin_per_unit, self.config.trim_pct);
        if safe_lots == 0 {
            return TickDecision {
                action: Action::DeadZone,
                close_side: None,
                close_qty: 0.0,
                reason: "No room to trim".to_string(),
            };
        }

        let close_qty = (safe_lots as f64).min(position.hedge_qty);
        if close_qty <= 0.0 {
            return TickDecision {
                action: Action::PureBias,
                close_side: None,
                close_qty: 0.0,
                reason: "No hedge lots remaining".to_string(),
            };
        }

        TickDecision {
            action: Action::Trim,
            close_side: Some(position.hedge_side().to_string()),
            close_qty,
            reason: format!(
                "ML {:.1}% > TRIM {:.1}% — close {:.0} {} (maxSafe={})",
                ml, self.config.trim_pct, close_qty, position.hedge_side(), safe_lots
            ),
        }
    }

    /// Calculate Open MG position size.
    /// Returns (per_side_qty, safe_gross).
    pub fn calc_open_mg_size(&self, equity: f64) -> (f64, f64) {
        let safe_gross = (equity / self.config.spread_tolerance).floor();
        let per_side = (safe_gross / 2.0).floor();
        (per_side, safe_gross)
    }
}
