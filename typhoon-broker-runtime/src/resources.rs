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

fn installed_memory_mb() -> u64 {
    std::fs::read_to_string("/proc/meminfo")
        .ok()
        .and_then(|meminfo| {
            meminfo.lines().find_map(|line| {
                let rest = line.strip_prefix("MemTotal:")?;
                rest.split_whitespace()
                    .next()
                    .and_then(|kb| kb.parse::<u64>().ok())
                    .map(|kb| kb / 1024)
            })
        })
        .unwrap_or(0)
}

fn memory_scaled_permits(base: usize, floor: usize) -> usize {
    let total_mb = installed_memory_mb();
    let pct = match total_mb {
        0 => 100,
        mb if mb <= 24_576 => 35,
        mb if mb <= 40_960 => 50,
        mb if mb <= 65_536 => 75,
        _ => 100,
    };
    if pct >= 100 || base <= floor {
        return base.max(floor);
    }
    base.saturating_mul(pct).div_ceil(100).max(floor).min(base)
}

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
        let yahoo_permits = memory_scaled_permits(6, 2);
        let kraken_public_permits = memory_scaled_permits(KRAKEN_PUBLIC_FETCH_PERMITS, 6);
        let kraken_equity_permits = memory_scaled_permits(KRAKEN_EQUITIES_FETCH_PERMITS, 2);
        Self {
            alpaca_fetch_permits: Arc::new(Semaphore::new(4)),
            // Yahoo's chart endpoint has no published rate limit but throttles
            // (429) and, on sustained abuse, temporarily IP-blocks. 4 was very
            // conservative; 6 gives the breadth-assist lane more parallelism. This
            // is the one lane with safe headroom — Kraken Spot (24 permits) sits
            // near its public counter and Kraken Equities (8) is pinned to the
            // ~6 req/s Cloudflare iapi wall, so neither can be pushed. If Yahoo
            // starts 429-ing/blocking, drop this back to 4.
            yahoo_chart_fetch_permits: Arc::new(Semaphore::new(yahoo_permits)),
            kraken_fetch_permits: Arc::new(Semaphore::new(kraken_public_permits)),
            kraken_equity_fetch_permits: Arc::new(Semaphore::new(kraken_equity_permits)),
            kraken_public_client: reqwest::Client::builder()
                .user_agent("TyphooN-Terminal/1.0")
                .pool_max_idle_per_host(kraken_public_permits * 2)
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
