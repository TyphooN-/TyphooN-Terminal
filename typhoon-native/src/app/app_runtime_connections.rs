use super::*;
use crate::app::app_runtime_support::is_routine_market_data_status;

impl TyphooNApp {
    pub(super) fn handle_broker_connected(&mut self, s: String) {
        if s.contains("Kraken") {
            if !self.kraken_enabled {
                return;
            }
            self.kraken_connected = true;
            self.resolve_order_broker(); // re-point routing only if current target is now unavailable
            // REST is authoritative: load balances/positions/history/orders before
            // relying on private WS deltas.
            let _ = self.broker_tx.send(BrokerCmd::KrakenGetBalance);
            let _ = self.broker_tx.send(BrokerCmd::KrakenGetPositions);
            let _ = self.broker_tx.send(BrokerCmd::KrakenFetchTrades);
            let _ = self.broker_tx.send(BrokerCmd::KrakenFetchOpenOrders);
            // Start private WebSocket for real-time ownTrades / openOrders.
            let _ = self.broker_tx.send(BrokerCmd::KrakenStartPrivateWs);
        } else {
            if !self.alpaca_enabled {
                return;
            }
            self.broker_connected = true;
            self.resolve_order_broker(); // re-point routing only if current target is now unavailable
            if self.alpaca_full_bar_sync_enabled {
                self.log.push_back(LogEntry::info(
                    "Alpaca connected — broad Alpaca universe bar sync enabled.",
                ));
            } else if self.backfill_alpaca_kraken_equities_enabled {
                self.log.push_back(LogEntry::info(
                    "Alpaca connected — Kraken assist only; broad Alpaca universe sync disabled.",
                ));
            } else {
                self.log.push_back(LogEntry::info(
                    "Alpaca connected — account/trading only; broad Alpaca universe sync disabled.",
                ));
            }
            // Auto-fetch positions, orders, and recent fills (Alpaca)
            let _ = self.broker_tx.send(BrokerCmd::GetPositions);
            let _ = self.broker_tx.send(BrokerCmd::GetOrders);
            let _ = self.broker_tx.send(BrokerCmd::GetActivities { limit: 100 });
            let _ = self.broker_tx.send(BrokerCmd::GetMarketClock);
            // Real-time order/fill/account updates over the trading WebSocket; the
            // periodic REST poll stays as a safety net for the reconnect window.
            let _ = self.broker_tx.send(BrokerCmd::AlpacaStartTradeStream);
        }
        if is_routine_market_data_status(&s) {
            tracing::debug!("{}", s);
        } else {
            self.log.push_back(LogEntry::info(s));
        }
    }
}
