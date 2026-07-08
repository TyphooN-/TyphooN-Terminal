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

    fn rebuild_live_positions_by_symbol(&mut self) {
        self.live_positions_by_symbol = self
            .live_positions
            .iter()
            .map(|p| {
                let key = bare_symbol_from_key(&p.symbol)
                    .replace("/", "")
                    .trim_end_matches(".EQ")
                    .trim_end_matches(".eq")
                    .to_ascii_uppercase();
                (key, p.clone())
            })
            .collect();
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
        self.rebuild_live_positions_by_symbol();
    }

    pub(super) fn handle_alpaca_account_positions(&mut self, accounts: Vec<AccountPositions>) {
        if !self.alpaca_enabled {
            return;
        }
        self.positions_last_update_ts = chrono::Utc::now().timestamp();
        let mut positions_map = std::collections::HashMap::with_capacity(accounts.len());
        let mut primary_account: Option<&AccountPositions> = None;
        for account in &accounts {
            if account.is_primary && primary_account.is_none() {
                primary_account = Some(account);
            }
            positions_map.insert(account.account_id.clone(), account.clone());
        }
        let pid: &str = self
            .alpaca_roster_by_id
            .get(&self.alpaca_primary_account_id)
            .map(|r| r.id.as_str())
            .unwrap_or(self.alpaca_primary_account_id.as_str());
        if let Some(primary) = positions_map.get(pid).cloned().or_else(|| primary_account.cloned()) {
            if let Ok(json) = serde_json::to_string(&primary.positions) {
                self.put_kv_dedup("broker:positions", &json);
            }
            self.live_positions = primary.positions.clone();
            self.rebuild_live_positions_by_symbol();
        }
        self.alpaca_account_positions = accounts;
        self.alpaca_account_positions_by_id = self
            .alpaca_account_positions
            .iter()
            .map(|a| (a.account_id.clone(), a.clone()))
            .collect();
    }

    pub(super) fn handle_alpaca_all_assets(&mut self, assets: Vec<AssetRow>) {
        if !self.alpaca_enabled {
            return;
        }
        self.all_broker_assets = assets;
        self.all_broker_assets_fetched = true;
        // Defer refill — the drain arm will set market_data_refill_requested
        // so heavy schedule_* work (rotations, kraken universes, alpaca pairs)
        // runs once at end of batch, outside individual msg timing.
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

    pub(super) fn handle_alpaca_account_fills(&mut self, accounts: Vec<AccountFills>) {
        if !self.alpaca_enabled {
            return;
        }
        let mut fills_map = std::collections::HashMap::with_capacity(accounts.len());
        let mut primary_account: Option<&AccountFills> = None;
        for account in &accounts {
            if account.is_primary && primary_account.is_none() {
                primary_account = Some(account);
            }
            fills_map.insert(account.account_id.clone(), account.clone());
        }
        let pid: &str = self.alpaca_primary_account_id.as_str();
        if let Some(primary) = fills_map
            .get(pid)
            .cloned()
            .or_else(|| primary_account.cloned())
        {
            // prefer primary_id map; fallback for legacy
            self.recent_fills = primary.fills;
        }
        self.alpaca_account_fills = accounts;
        self.alpaca_account_fills_by_id = self
            .alpaca_account_fills
            .iter()
            .map(|a| (a.account_id.clone(), a.clone()))
            .collect();
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
        self.live_orders_by_id = self
            .live_orders
            .iter()
            .map(|o| (o.id.clone(), o.clone()))
            .collect();
    }

    pub(super) fn handle_alpaca_account_orders(&mut self, accounts: Vec<AccountOrders>) {
        if !self.alpaca_enabled {
            return;
        }
        self.orders_last_update_ts = chrono::Utc::now().timestamp();
        let mut orders_map = std::collections::HashMap::with_capacity(accounts.len());
        let mut primary_account: Option<&AccountOrders> = None;
        for account in &accounts {
            if account.is_primary && primary_account.is_none() {
                primary_account = Some(account);
            }
            orders_map.insert(account.account_id.clone(), account.clone());
        }
        let pid: &str = self.alpaca_primary_account_id.as_str();
        if let Some(primary) = orders_map
            .get(pid)
            .cloned()
            .or_else(|| primary_account.cloned())
        {
            // prefer primary_id map; fallback for legacy
            self.live_orders = primary.orders;
            self.live_orders_by_id = self
                .live_orders
                .iter()
                .map(|o| (o.id.clone(), o.clone()))
                .collect();
        }
        self.alpaca_account_orders = accounts;
        self.alpaca_account_orders_by_id = self
            .alpaca_account_orders
            .iter()
            .map(|a| (a.account_id.clone(), a.clone()))
            .collect();
    }
}
