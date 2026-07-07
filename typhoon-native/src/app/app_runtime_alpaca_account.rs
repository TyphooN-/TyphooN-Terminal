use super::*;
type AssetRow = (String, String, String);
type RecentFillRow = (String, String, f64, f64, String);

impl TyphooNApp {
    pub(super) fn handle_alpaca_account(&mut self, acct: AccountInfo) {
        if !self.alpaca_enabled {
            return;
        }
        // Store to cache KV — dedup to avoid timestamp churn
        if let Ok(json) = serde_json::to_string(&acct) {
            self.put_kv_dedup("broker:account", &json);
        }
        self.live_account = Some(acct);
    }

    pub(super) fn handle_alpaca_positions(&mut self, pos: Vec<PositionInfo>) {
        if !self.alpaca_enabled {
            return;
        }
        self.positions_last_update_ts = chrono::Utc::now().timestamp();
        if let Ok(json) = serde_json::to_string(&pos) {
            self.put_kv_dedup("broker:positions", &json);
        }
        self.live_positions = pos;
    }

    pub(super) fn handle_alpaca_account_positions(&mut self, accounts: Vec<AccountPositions>) {
        if !self.alpaca_enabled {
            return;
        }
        self.positions_last_update_ts = chrono::Utc::now().timestamp();
        if let Some(primary) = accounts.iter().find(|account| account.is_primary) {
            if let Ok(json) = serde_json::to_string(&primary.positions) {
                self.put_kv_dedup("broker:positions", &json);
            }
            self.live_positions = primary.positions.clone();
        }
        self.alpaca_account_positions = accounts;
    }

    pub(super) fn handle_alpaca_all_assets(&mut self, assets: Vec<AssetRow>) {
        if !self.alpaca_enabled {
            return;
        }
        self.all_broker_assets = assets;
        self.all_broker_assets_fetched = true;
    }

    pub(super) fn handle_alpaca_recent_fills(&mut self, fills: Vec<RecentFillRow>) {
        if !self.alpaca_enabled {
            return;
        }
        self.recent_fills = fills;
        // Invalidate trade overlay cache so fills show immediately
        for c in &mut self.charts {
            c.cached_trade_overlay_frame = 0;
        }
    }

    pub(super) fn handle_alpaca_orders(&mut self, orders: Vec<OrderInfo>) {
        if !self.alpaca_enabled {
            return;
        }
        self.orders_last_update_ts = chrono::Utc::now().timestamp();
        if let Ok(json) = serde_json::to_string(&orders) {
            self.put_kv_dedup("broker:orders", &json);
        }
        self.live_orders = orders;
    }

    pub(super) fn handle_alpaca_account_orders(&mut self, accounts: Vec<AccountOrders>) {
        if !self.alpaca_enabled {
            return;
        }
        self.orders_last_update_ts = chrono::Utc::now().timestamp();
        if let Some(primary) = accounts.iter().find(|account| account.is_primary) {
            self.live_orders = primary.orders.clone();
        }
        self.alpaca_account_orders = accounts;
    }
}
