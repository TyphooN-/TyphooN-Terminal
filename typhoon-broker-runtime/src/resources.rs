//! Broker runtime resource construction.
//!
//! Keep lower-layer concurrency permits and HTTP clients out of the native app
//! shell. The native crate still owns UI/app state, but runtime resource setup now
//! lives in the crate that will own the broker processor after ADR-125 Target 3.

use std::sync::Arc;

use tokio::sync::Semaphore;
use typhoon_engine::broker::sync_config::{
    KRAKEN_EQUITIES_FETCH_PERMITS, KRAKEN_PUBLIC_FETCH_PERMITS,
};

#[derive(Clone)]
pub struct BrokerRuntimeResources {
    pub alpaca_fetch_permits: Arc<Semaphore>,
    pub yahoo_chart_fetch_permits: Arc<Semaphore>,
    pub kraken_fetch_permits: Arc<Semaphore>,
    pub kraken_equity_fetch_permits: Arc<Semaphore>,
    pub kraken_public_client: reqwest::Client,
    pub fallback_bar_client: reqwest::Client,
}

impl BrokerRuntimeResources {
    pub fn new() -> Self {
        Self {
            alpaca_fetch_permits: Arc::new(Semaphore::new(4)),
            yahoo_chart_fetch_permits: Arc::new(Semaphore::new(4)),
            kraken_fetch_permits: Arc::new(Semaphore::new(KRAKEN_PUBLIC_FETCH_PERMITS)),
            kraken_equity_fetch_permits: Arc::new(Semaphore::new(KRAKEN_EQUITIES_FETCH_PERMITS)),
            kraken_public_client: reqwest::Client::builder()
                .user_agent("TyphooN-Terminal/1.0")
                .pool_max_idle_per_host(KRAKEN_PUBLIC_FETCH_PERMITS * 2)
                .build()
                .unwrap_or_default(),
            fallback_bar_client: reqwest::Client::builder()
                .user_agent("TyphooN-Terminal/1.0")
                .pool_max_idle_per_host(8)
                .timeout(std::time::Duration::from_secs(20))
                .build()
                .unwrap_or_default(),
        }
    }
}

impl Default for BrokerRuntimeResources {
    fn default() -> Self {
        Self::new()
    }
}
