use serde::{Deserialize, Serialize};

// ── Client → Server ─────────────────────────────────────────────────
#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(tag = "type")]
pub enum WebCmd {
    GetAccount,
    GetPositions,
    GetOrders,
    GetWatchlistQuotes { symbols: Vec<String> },
    GetBars { symbol: String, timeframe: String },
    GetMarketClock,
    Ping,
}

// ── Server → Client ─────────────────────────────────────────────────
#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(tag = "type")]
pub enum WebMsg {
    Account(AccountSnapshot),
    Positions { items: Vec<PositionSnapshot> },
    Orders { items: Vec<OrderSnapshot> },
    WatchlistQuotes { items: Vec<QuoteSnapshot> },
    Bars {
        symbol: String,
        timeframe: String,
        bars: Vec<BarData>,
    },
    MarketClock { info: String },
    QuoteTick {
        symbol: String,
        bid: f64,
        ask: f64,
    },
    Error { msg: String },
    Pong,
}

// ── Snapshot types ──────────────────────────────────────────────────
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct AccountSnapshot {
    pub equity: f64,
    pub cash: f64,
    pub buying_power: f64,
    pub portfolio_value: f64,
    pub unrealized_pl: f64,
    pub initial_margin: f64,
    pub maintenance_margin: f64,
    pub currency: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct PositionSnapshot {
    pub symbol: String,
    pub qty: f64,
    pub side: String,
    pub avg_entry_price: f64,
    pub market_value: f64,
    pub unrealized_pl: f64,
    pub asset_class: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct OrderSnapshot {
    pub id: String,
    pub symbol: String,
    pub qty: String,
    pub side: String,
    pub order_type: String,
    pub status: String,
    pub limit_price: Option<String>,
    pub stop_price: Option<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct QuoteSnapshot {
    pub symbol: String,
    pub last: f64,
    pub bid: f64,
    pub ask: f64,
    pub change_pct: f64,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct BarData {
    pub timestamp: i64,
    pub open: f64,
    pub high: f64,
    pub low: f64,
    pub close: f64,
    pub volume: f64,
}
