use serde::{Deserialize, Serialize};

// ── Data Types ──────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DarwinAccount {
    pub name: String,          // MT5 name (e.g. "TyphooN_MT5")
    pub darwin_ticker: String, // Account ID from filename (e.g. "XUQF" for Darwinex, "MAIN" for other MT5)
    pub mt5_account: String,   // MT5 account number
    pub initial_balance: f64,
    pub created_at: i64, // import timestamp
    pub deal_count: i64,
    pub position_count: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DarwinDeal {
    pub id: i64,
    pub account: String, // darwin_ticker
    pub time: String,    // "2024.10.08 16:47:19"
    pub deal_ticket: i64,
    pub symbol: String,
    pub deal_type: String, // "buy", "sell", "balance"
    pub direction: String, // "in", "out", ""
    pub volume: f64,
    pub price: f64,
    pub order_ticket: i64,
    pub commission: f64,
    pub fee: f64,
    pub swap: f64,
    pub profit: f64,
    pub balance: f64,
    pub comment: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DarwinPosition {
    pub id: i64,
    pub account: String,
    pub open_time: String,
    pub position_ticket: i64,
    pub symbol: String,
    pub pos_type: String, // "buy", "sell"
    pub volume: f64,
    pub open_price: f64,
    pub sl: f64,
    pub tp: f64,
    pub close_time: String,
    pub close_price: f64,
    pub commission: f64,
    pub swap: f64,
    pub profit: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DarwinAccountSummary {
    pub account: DarwinAccount,
    pub total_profit: f64,
    pub total_commission: f64,
    pub total_swap: f64,
    pub win_count: i64,
    pub loss_count: i64,
    pub win_rate: f64,
    pub profit_factor: f64,
    pub final_balance: f64,
    pub max_drawdown_pct: f64,
    pub symbols_traded: Vec<String>,
}
