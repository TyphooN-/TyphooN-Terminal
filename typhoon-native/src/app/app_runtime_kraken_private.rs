use super::*;
use std::collections::HashMap;
use typhoon_engine::broker::kraken::{KrakenOrder, KrakenTrade};

impl TyphooNApp {
    pub(super) fn handle_kraken_trades(&mut self, mut trades: Vec<KrakenTrade>) {
        if !self.kraken_enabled {
            return;
        }
        trades.sort_by(|a, b| {
            b.time
                .partial_cmp(&a.time)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        if trades.len() > KRAKEN_TRADE_HISTORY_CAP {
            trades.truncate(KRAKEN_TRADE_HISTORY_CAP);
        }
        let prev_trades = self.kraken_trades.len();
        let prev_basis = self.kraken_cost_basis.len();
        self.kraken_trades = VecDeque::from(trades);
        self.rebuild_kraken_trade_indexes();
        self.refresh_kraken_position_costs();
        for c in &mut self.charts {
            c.cached_trade_overlay_frame = 0;
        }
        self.kraken_trades_last_fetch = std::time::Instant::now();
        let new_trades = self.kraken_trades.len();
        let new_basis = self.kraken_cost_basis.len();
        // The safety-net REST fetch normally returns the same counts as the last
        // pull. Only surface the user log line when something actually changed;
        // routine confirmations go to trace at debug level.
        if new_trades != prev_trades || new_basis != prev_basis {
            self.log.push_back(LogEntry::info(format!(
                "Kraken: loaded {} trades; cost basis for {} held assets",
                new_trades, new_basis
            )));
        } else {
            tracing::debug!(
                "Kraken trades resync: {} trades / {} held assets (unchanged)",
                new_trades,
                new_basis
            );
        }
    }

    pub(super) fn handle_kraken_account_trades(&mut self, mut accounts: Vec<KrakenAccountTrades>) {
        if !self.kraken_enabled {
            return;
        }
        for account in &mut accounts {
            account.trades.sort_by(|a, b| {
                b.time
                    .partial_cmp(&a.time)
                    .unwrap_or(std::cmp::Ordering::Equal)
            });
        }
        let acc_map: std::collections::HashMap<_, _> = accounts.iter().map(|a| (a.account_id.clone(), a.clone())).collect();
        if let Some(primary) = acc_map.get(&self.kraken_primary_account_id).or_else(|| accounts.iter().find(|a| a.is_primary)) {
            self.handle_kraken_trades(primary.trades.clone());
        }
        self.kraken_account_trades = accounts;
        self.kraken_account_trades_by_id = self.kraken_account_trades.iter().map(|a| (a.account_id.clone(), a.clone())).collect();
    }

    pub(super) fn handle_kraken_live_trade(&mut self, trade: KrakenTrade) {
        if !self.kraken_enabled {
            return;
        }
        let t0 = std::time::Instant::now();
        let inserted = self.insert_kraken_live_trade(trade);
        if inserted {
            self.refresh_kraken_position_costs();
            for c in &mut self.charts {
                c.cached_trade_overlay_frame = 0;
            }
            let _ = self.broker_tx.send(BrokerCmd::KrakenGetBalance);
            let _ = self.broker_tx.send(BrokerCmd::KrakenGetPositions);
            let _ = self.broker_tx.send(BrokerCmd::KrakenFetchOpenOrders);
        }
        let dt = t0.elapsed();
        if dt > std::time::Duration::from_millis(2) {
            tracing::warn!("KrakenLiveTrade path took {:?} (inserted={})", dt, inserted);
        }
    }

    pub(super) fn handle_kraken_positions(&mut self, mut pos: Vec<PositionInfo>) {
        if !self.kraken_enabled {
            return;
        }
        self.positions_last_update_ts = chrono::Utc::now().timestamp();
        pos.retain(|p| p.asset_class != "crypto_spot" && !p.asset_id.starts_with("spot:"));
        if let Ok(json) = serde_json::to_string(&pos) {
            self.put_kv_dedup("broker:kr_positions", &json);
        }
        self.kr_positions = pos;
        self.kr_positions_by_symbol = self.kr_positions.iter().map(|p| {
            let key = bare_symbol_from_key(&p.symbol).replace("/", "").trim_end_matches(".EQ").trim_end_matches(".eq").to_ascii_uppercase();
            (key, p.clone())
        }).collect();
        self.refresh_kraken_position_costs();
        for c in &mut self.charts {
            c.cached_trade_overlay_frame = 0;
        }
    }

    pub(super) fn handle_kraken_account_positions(
        &mut self,
        accounts: Vec<KrakenAccountPositions>,
    ) {
        if !self.kraken_enabled {
            return;
        }
        self.positions_last_update_ts = chrono::Utc::now().timestamp();
        let acc_map: std::collections::HashMap<_, _> = accounts.iter().map(|a| (a.account_id.clone(), a.clone())).collect();
        if let Some(primary) = acc_map.get(&self.kraken_primary_account_id).or_else(|| accounts.iter().find(|a| a.is_primary)) {
            self.kr_positions = primary.positions.clone();
            self.kr_positions_by_symbol = self.kr_positions.iter().map(|p| {
                let key = bare_symbol_from_key(&p.symbol).replace("/", "").trim_end_matches(".EQ").trim_end_matches(".eq").to_ascii_uppercase();
                (key, p.clone())
            }).collect();
        }
        self.kraken_account_positions = accounts;
        self.kraken_account_positions_by_id = self.kraken_account_positions.iter().map(|a| (a.account_id.clone(), a.clone())).collect();
        self.refresh_kraken_position_costs();
        for c in &mut self.charts {
            c.cached_trade_overlay_frame = 0;
        }
    }

    pub(super) fn handle_kraken_open_orders(&mut self, orders: Vec<KrakenOrder>) {
        if !self.kraken_enabled {
            return;
        }

        let mut by_txid: HashMap<String, KrakenOrder> = self
            .kraken_open_orders
            .iter()
            .cloned()
            .map(|order| (order.txid.clone(), order))
            .collect();

        for order in orders {
            let terminal = matches!(
                order.status.as_str(),
                "closed" | "canceled" | "cancelled" | "expired"
            );
            if terminal {
                by_txid.remove(&order.txid);
            } else {
                by_txid.insert(order.txid.clone(), order);
            }
        }

        self.kraken_open_orders = by_txid.into_values().collect();
        self.kraken_open_orders.sort_by(|a, b| {
            b.opentm
                .partial_cmp(&a.opentm)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
    }

    pub(super) fn handle_kraken_account_open_orders(
        &mut self,
        mut accounts: Vec<KrakenAccountOrders>,
    ) {
        if !self.kraken_enabled {
            return;
        }
        for account in &mut accounts {
            account.orders.sort_by(|a, b| {
                b.opentm
                    .partial_cmp(&a.opentm)
                    .unwrap_or(std::cmp::Ordering::Equal)
            });
        }
        let acc_map: std::collections::HashMap<_, _> = accounts.iter().map(|a| (a.account_id.clone(), a.clone())).collect();
        if let Some(primary) = acc_map.get(&self.kraken_primary_account_id).or_else(|| accounts.iter().find(|a| a.is_primary)) {
            self.kraken_open_orders = primary.orders.clone();
        }
        self.kraken_account_orders = accounts;
        self.kraken_account_orders_by_id = self.kraken_account_orders.iter().map(|a| (a.account_id.clone(), a.clone())).collect();
    }
}
