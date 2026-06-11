use super::*;
type AssetRow = (String, String, String);
type RecentFillRow = (String, String, f64, f64, String);

impl TyphooNApp {
    pub(super) fn handle_alpaca_account(&mut self, acct: AccountInfo) {
        if !self.alpaca_enabled {
            return;
        }
        // Store to cache KV for LAN sync — dedup to avoid timestamp churn
        if let Ok(json) = serde_json::to_string(&acct) {
            self.put_kv_dedup("broker:account", &json);
        }
        // Broadcast to web clients
        if let Some(ref tx) = self.web_msg_tx {
            let _ = tx.send(typhoon_web_protocol::WebMsg::Account(
                typhoon_web_protocol::AccountSnapshot {
                    equity: acct.equity,
                    cash: acct.cash,
                    buying_power: acct.buying_power,
                    portfolio_value: acct.portfolio_value,
                    unrealized_pl: 0.0, // computed from positions
                    initial_margin: acct.initial_margin,
                    maintenance_margin: acct.maintenance_margin,
                    currency: acct.currency.clone(),
                },
            ));
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
        // Broadcast to web clients
        if let Some(ref tx) = self.web_msg_tx {
            let items: Vec<typhoon_web_protocol::PositionSnapshot> = pos
                .iter()
                .map(|p| typhoon_web_protocol::PositionSnapshot {
                    symbol: p.symbol.clone(),
                    qty: p.qty,
                    side: p.side.clone(),
                    avg_entry_price: p.avg_entry_price,
                    market_value: p.market_value,
                    unrealized_pl: p.unrealized_pl,
                    asset_class: p.asset_class.clone(),
                })
                .collect();
            let _ = tx.send(typhoon_web_protocol::WebMsg::Positions { items });
        }
        self.live_positions = pos;
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
        // Broadcast to web clients
        if let Some(ref tx) = self.web_msg_tx {
            let items: Vec<typhoon_web_protocol::OrderSnapshot> = orders
                .iter()
                .map(|o| typhoon_web_protocol::OrderSnapshot {
                    id: o.id.clone(),
                    symbol: o.symbol.clone(),
                    qty: o.qty.clone(),
                    side: o.side.clone(),
                    order_type: o.order_type.clone(),
                    status: o.status.clone(),
                    limit_price: o.limit_price.clone(),
                    stop_price: o.stop_price.clone(),
                })
                .collect();
            let _ = tx.send(typhoon_web_protocol::WebMsg::Orders { items });
        }
        self.live_orders = orders;
    }
}
