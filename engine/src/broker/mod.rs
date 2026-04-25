pub mod alpaca;
pub mod dxlink;
pub mod kraken_broker;
pub mod tastytrade;

use alpaca::{AccountInfo, AssetInfo, Bar, OrderInfo, OrderResult, PositionInfo};
use async_trait::async_trait;

/// Broker abstraction trait. AlpacaBroker implements this.
#[async_trait]
pub trait BrokerTrait: Send + Sync {
    /// Get account info (equity, balance, margin, etc.).
    async fn get_account(&self) -> Result<AccountInfo, String>;

    /// Get all open positions.
    async fn get_positions(&self) -> Result<Vec<PositionInfo>, String>;

    /// Place a market order.
    async fn market_order(&self, symbol: &str, qty: f64, side: &str)
    -> Result<OrderResult, String>;

    /// Place a limit order.
    async fn limit_order(
        &self,
        symbol: &str,
        qty: f64,
        side: &str,
        limit_price: f64,
        tif: &str,
    ) -> Result<OrderResult, String>;

    /// Close a position (fully or partially).
    async fn close_position(&self, symbol: &str, qty: Option<f64>) -> Result<OrderResult, String>;

    /// Get historical bar data.
    async fn get_bars(&self, symbol: &str, timeframe: &str, limit: u32)
    -> Result<Vec<Bar>, String>;

    /// Get news for a symbol.
    async fn get_news(&self, symbol: &str, limit: u32) -> Result<Vec<serde_json::Value>, String>;

    /// Get orders by status.
    async fn get_orders(&self, status: &str, limit: u32) -> Result<Vec<OrderInfo>, String>;

    /// Get asset info.
    async fn get_asset(&self, symbol: &str) -> Result<AssetInfo, String>;
}

#[async_trait]
impl BrokerTrait for alpaca::AlpacaBroker {
    async fn get_account(&self) -> Result<AccountInfo, String> {
        self.get_account().await
    }

    async fn get_positions(&self) -> Result<Vec<PositionInfo>, String> {
        self.get_positions().await
    }

    async fn market_order(
        &self,
        symbol: &str,
        qty: f64,
        side: &str,
    ) -> Result<OrderResult, String> {
        self.market_order(symbol, qty, side).await
    }

    async fn limit_order(
        &self,
        symbol: &str,
        qty: f64,
        side: &str,
        limit_price: f64,
        tif: &str,
    ) -> Result<OrderResult, String> {
        self.limit_order(symbol, qty, side, limit_price, tif).await
    }

    async fn close_position(&self, symbol: &str, qty: Option<f64>) -> Result<OrderResult, String> {
        self.close_position(symbol, qty).await
    }

    async fn get_bars(
        &self,
        symbol: &str,
        timeframe: &str,
        limit: u32,
    ) -> Result<Vec<Bar>, String> {
        self.get_bars(symbol, timeframe, limit).await
    }

    async fn get_news(&self, symbol: &str, limit: u32) -> Result<Vec<serde_json::Value>, String> {
        self.get_news(symbol, limit).await
    }

    async fn get_orders(&self, status: &str, limit: u32) -> Result<Vec<OrderInfo>, String> {
        self.get_orders(status, limit).await
    }

    async fn get_asset(&self, symbol: &str) -> Result<AssetInfo, String> {
        self.get_asset(symbol).await
    }
}
