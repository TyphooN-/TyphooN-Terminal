//! Multi-account broker pools (ADR-130).
//!
//! One broker module can hold several logged-in accounts. The pool owns the
//! per-account client instances and answers two routing questions:
//!
//! * **Trading / account data** → the `primary` account (top-bar cycler).
//! * **Historical bar sync** → round-robin over every `data_sync_enabled`
//!   account. Each `AlpacaBroker` carries its own rate limiter, so N accounts
//!   multiply the aggregate historical request budget without any account
//!   exceeding its individual per-key limit.

use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

use typhoon_engine::broker::alpaca::AlpacaBroker;
use typhoon_engine::broker::kraken::KrakenBroker;
use typhoon_engine::broker::protocol::{AccountRosterEntry, BrokerAccountSpec};

pub struct AlpacaAccountHandle {
    pub spec: BrokerAccountSpec,
    pub broker: AlpacaBroker,
    pub equity: f64,
    pub connected: bool,
    pub detail: String,
}

#[derive(Default)]
pub struct AlpacaAccountPool {
    accounts: Vec<AlpacaAccountHandle>,
    primary_idx: usize,
    data_cursor: Arc<AtomicUsize>,
    mirror_orders: bool,
    /// Explicit opt-in set for live order mirroring (TradeCopy window
    /// checkboxes). Always starts empty — never persisted.
    mirror_target_ids: std::collections::BTreeSet<String>,
}

impl AlpacaAccountPool {
    pub fn new(accounts: Vec<AlpacaAccountHandle>, primary_id: &str) -> Self {
        let primary_idx = accounts
            .iter()
            .position(|a| a.spec.id == primary_id && a.connected)
            .or_else(|| accounts.iter().position(|a| a.connected))
            .unwrap_or(0);
        Self {
            accounts,
            primary_idx,
            data_cursor: Arc::new(AtomicUsize::new(0)),
            mirror_orders: false,
            mirror_target_ids: std::collections::BTreeSet::new(),
        }
    }

    pub fn is_empty(&self) -> bool {
        !self.accounts.iter().any(|a| a.connected)
    }

    /// Trading / account-data client (the account the UI is "on").
    pub fn primary_broker(&self) -> Option<&AlpacaBroker> {
        self.accounts
            .get(self.primary_idx)
            .filter(|a| a.connected)
            .map(|a| &a.broker)
    }

    pub fn primary_id(&self) -> Option<&str> {
        self.accounts
            .get(self.primary_idx)
            .map(|a| a.spec.id.as_str())
    }

    /// Switch primary by account id. Returns true when the primary changed to
    /// a connected account.
    pub fn set_primary(&mut self, account_id: &str) -> bool {
        if let Some(idx) = self
            .accounts
            .iter()
            .position(|a| a.spec.id == account_id && a.connected)
        {
            let changed = idx != self.primary_idx;
            self.primary_idx = idx;
            changed
        } else {
            false
        }
    }

    /// Round-robin data-sync account for the next historical bar fetch.
    /// Falls back to the primary when no account is data-sync enabled.
    pub fn next_data_broker(&self) -> Option<AlpacaBroker> {
        let data_accounts: Vec<&AlpacaAccountHandle> = self
            .accounts
            .iter()
            .filter(|a| a.connected && a.spec.data_sync_enabled)
            .collect();
        if data_accounts.is_empty() {
            return self.primary_broker().cloned();
        }
        let idx = self.data_cursor.fetch_add(1, Ordering::Relaxed) % data_accounts.len();
        Some(data_accounts[idx].broker.clone())
    }

    pub fn data_account_count(&self) -> usize {
        let n = self
            .accounts
            .iter()
            .filter(|a| a.connected && a.spec.data_sync_enabled)
            .count();
        n.max(usize::from(!self.is_empty()))
    }

    /// Explicitly opted-in mirror targets: accounts the user checked in the
    /// TradeCopy window, minus the primary. An empty opt-in set yields no
    /// targets — mirroring never fans out to accounts by default.
    pub fn mirror_targets(&self) -> Vec<(&BrokerAccountSpec, &AlpacaBroker)> {
        self.accounts
            .iter()
            .enumerate()
            .filter(|(idx, a)| {
                *idx != self.primary_idx
                    && a.connected
                    && a.spec.trade_enabled
                    && self.mirror_target_ids.contains(&a.spec.id)
            })
            .map(|(_, a)| (&a.spec, &a.broker))
            .collect()
    }

    pub fn broker_by_id(&self, account_id: &str) -> Option<&AlpacaAccountHandle> {
        self.accounts
            .iter()
            .find(|a| a.spec.id == account_id && a.connected)
    }

    pub fn set_mirror_orders(&mut self, enabled: bool, target_ids: Vec<String>) {
        self.mirror_target_ids = target_ids.into_iter().collect();
        // Enabling with nothing opted in is a no-op kept off for safety.
        self.mirror_orders = enabled && !self.mirror_target_ids.is_empty();
    }

    pub fn mirror_orders(&self) -> bool {
        self.mirror_orders
    }

    pub async fn apply_bar_rpm_hint(&self, rpm: u32) {
        for account in &self.accounts {
            account.broker.set_bar_requests_per_minute_hint(rpm).await;
        }
    }

    pub fn roster(&self) -> Vec<AccountRosterEntry> {
        self.accounts
            .iter()
            .enumerate()
            .map(|(idx, a)| AccountRosterEntry {
                id: a.spec.id.clone(),
                label: a.spec.label.clone(),
                paper: a.spec.paper,
                trade_enabled: a.spec.trade_enabled,
                data_sync_enabled: a.spec.data_sync_enabled,
                equity: a.equity,
                is_primary: idx == self.primary_idx,
                connected: a.connected,
                detail: a.detail.clone(),
            })
            .collect()
    }
}

pub struct KrakenAccountHandle {
    pub spec: BrokerAccountSpec,
    pub broker: KrakenBroker,
    pub connected: bool,
    pub detail: String,
}

#[derive(Default)]
pub struct KrakenAccountPool {
    accounts: Vec<KrakenAccountHandle>,
    primary_idx: usize,
    /// Dedicated WS-token key pair, if the user configured one. Applies to the
    /// first (kraken1) account only; other accounts authenticate WS with their
    /// REST keys.
    ws_override: Option<(String, String)>,
}

impl KrakenAccountPool {
    pub fn new(accounts: Vec<KrakenAccountHandle>, primary_id: &str) -> Self {
        let primary_idx = accounts
            .iter()
            .position(|a| a.spec.id == primary_id && a.connected)
            .or_else(|| accounts.iter().position(|a| a.connected))
            .unwrap_or(0);
        Self {
            accounts,
            primary_idx,
            ws_override: None,
        }
    }

    pub fn set_ws_override(&mut self, ws_key: String, ws_secret: String) {
        if !ws_key.trim().is_empty() && !ws_secret.trim().is_empty() {
            self.ws_override = Some((ws_key, ws_secret));
        }
    }

    /// Key pair the WS-token broker should use for the current primary: the
    /// dedicated WS override for the first account, REST keys otherwise.
    pub fn ws_keys_for_primary(&self) -> Option<(String, String)> {
        let account = self.accounts.get(self.primary_idx)?;
        if self.primary_idx == 0 {
            if let Some(ws) = self.ws_override.clone() {
                return Some(ws);
            }
        }
        Some((account.spec.api_key.clone(), account.spec.secret.clone()))
    }

    pub fn primary_broker(&self) -> Option<&KrakenBroker> {
        self.accounts
            .get(self.primary_idx)
            .filter(|a| a.connected)
            .map(|a| &a.broker)
    }

    pub fn primary_id(&self) -> Option<&str> {
        self.accounts
            .get(self.primary_idx)
            .map(|a| a.spec.id.as_str())
    }

    pub fn broker_by_id(&self, account_id: &str) -> Option<&KrakenAccountHandle> {
        self.accounts
            .iter()
            .find(|a| a.spec.id == account_id && a.connected)
    }

    pub fn set_primary(&mut self, account_id: &str) -> bool {
        if let Some(idx) = self
            .accounts
            .iter()
            .position(|a| a.spec.id == account_id && a.connected)
        {
            let changed = idx != self.primary_idx;
            self.primary_idx = idx;
            changed
        } else {
            false
        }
    }

    pub fn roster(&self) -> Vec<AccountRosterEntry> {
        self.accounts
            .iter()
            .enumerate()
            .map(|(idx, a)| AccountRosterEntry {
                id: a.spec.id.clone(),
                label: a.spec.label.clone(),
                paper: false,
                trade_enabled: a.spec.trade_enabled,
                data_sync_enabled: false,
                equity: 0.0,
                is_primary: idx == self.primary_idx,
                connected: a.connected,
                detail: a.detail.clone(),
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn spec(id: &str, data: bool, trade: bool) -> BrokerAccountSpec {
        BrokerAccountSpec {
            id: id.to_string(),
            label: id.to_uppercase(),
            api_key: "k".into(),
            secret: "s".into(),
            paper: true,
            trade_enabled: trade,
            data_sync_enabled: data,
        }
    }

    fn handle_with_rpm(
        id: &str,
        data: bool,
        trade: bool,
        connected: bool,
        rpm: u32,
    ) -> AlpacaAccountHandle {
        AlpacaAccountHandle {
            spec: spec(id, data, trade),
            broker: AlpacaBroker::new("k".into(), "s".into(), true, rpm),
            equity: 0.0,
            connected,
            detail: String::new(),
        }
    }

    fn handle(id: &str, data: bool, trade: bool, connected: bool) -> AlpacaAccountHandle {
        handle_with_rpm(id, data, trade, connected, 200)
    }

    #[test]
    fn round_robin_covers_all_data_sync_accounts() {
        // Distinct per-account RPMs let the returned clone identify which
        // account served the draw.
        let pool = AlpacaAccountPool::new(
            vec![
                handle_with_rpm("alpaca1", true, true, true, 300),
                handle_with_rpm("alpaca2", true, false, true, 400),
                handle_with_rpm("alpaca3", false, true, true, 500),
                handle_with_rpm("alpaca4", true, false, true, 600),
            ],
            "alpaca1",
        );
        assert_eq!(pool.data_account_count(), 3);
        let draws: Vec<u32> = (0..6)
            .map(|_| {
                pool.next_data_broker()
                    .expect("data broker")
                    .bar_requests_per_minute()
            })
            .collect();
        assert_eq!(draws, vec![300, 400, 600, 300, 400, 600]);
    }

    #[test]
    fn primary_switch_requires_connected_account() {
        let mut pool = AlpacaAccountPool::new(
            vec![
                handle("alpaca1", true, true, true),
                handle("alpaca2", true, true, false),
                handle("alpaca3", true, true, true),
            ],
            "alpaca1",
        );
        assert_eq!(pool.primary_id(), Some("alpaca1"));
        assert!(!pool.set_primary("alpaca2"), "disconnected account refused");
        assert_eq!(pool.primary_id(), Some("alpaca1"));
        assert!(pool.set_primary("alpaca3"));
        assert_eq!(pool.primary_id(), Some("alpaca3"));
    }

    #[test]
    fn mirror_targets_are_strictly_opt_in() {
        let mut pool = AlpacaAccountPool::new(
            vec![
                handle("alpaca1", true, true, true),
                handle("alpaca2", true, true, true),
                handle("alpaca3", true, false, true),
            ],
            "alpaca1",
        );
        // Nothing opted in → nothing mirrors, even with trade-enabled accounts.
        assert!(pool.mirror_targets().is_empty());
        assert!(!pool.mirror_orders());

        // Enabling with an empty opt-in set stays off for safety.
        pool.set_mirror_orders(true, Vec::new());
        assert!(!pool.mirror_orders());

        // Opting in alpaca2 mirrors exactly alpaca2; the primary and
        // non-opted-in accounts stay excluded.
        pool.set_mirror_orders(true, vec!["alpaca1".into(), "alpaca2".into()]);
        assert!(pool.mirror_orders());
        let targets: Vec<&str> = pool
            .mirror_targets()
            .into_iter()
            .map(|(spec, _)| spec.id.as_str())
            .collect();
        assert_eq!(targets, vec!["alpaca2"]);
    }

    #[test]
    fn disconnected_primary_falls_back_to_first_connected() {
        let pool = AlpacaAccountPool::new(
            vec![
                handle("alpaca1", true, true, false),
                handle("alpaca2", true, true, true),
            ],
            "alpaca1",
        );
        assert_eq!(pool.primary_id(), Some("alpaca2"));
    }
}
