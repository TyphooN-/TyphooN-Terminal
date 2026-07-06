//! Lightweight process/system memory pressure gates for broker-side broad sync.
//!
//! The native scheduler already stops queueing broad work when RSS climbs, but
//! full-history HTTP responses can overshoot by multiple GB between UI ticks. This
//! broker-side gate sits immediately before expensive network fetches so queued
//! broad jobs wait for headroom instead of starting another response/parse/cache
//! burst while the process is already near the OOM cliff.

use std::time::Duration;

use typhoon_engine::broker::protocol::BrokerMsg;

const MAX_HEADROOM_WAIT: Duration = Duration::from_secs(120);
const CHECK_INTERVAL: Duration = Duration::from_millis(750);

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct MemorySnapshot {
    pub rss_mb: u64,
    pub total_mb: u64,
    pub available_mb: u64,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BrokerMemoryPressure {
    Normal,
    Reduced,
    PauseBroadFetches,
}

pub fn current_memory_snapshot() -> MemorySnapshot {
    let mut snapshot = MemorySnapshot::default();
    if let Ok(status) = std::fs::read_to_string("/proc/self/status") {
        for line in status.lines() {
            if let Some(rest) = line.strip_prefix("VmRSS:") {
                snapshot.rss_mb = rest
                    .split_whitespace()
                    .next()
                    .and_then(|kb| kb.parse::<u64>().ok())
                    .map(|kb| kb / 1024)
                    .unwrap_or(0);
                break;
            }
        }
    }
    if let Ok(meminfo) = std::fs::read_to_string("/proc/meminfo") {
        for line in meminfo.lines() {
            if let Some(rest) = line.strip_prefix("MemTotal:") {
                snapshot.total_mb = rest
                    .split_whitespace()
                    .next()
                    .and_then(|kb| kb.parse::<u64>().ok())
                    .map(|kb| kb / 1024)
                    .unwrap_or(0);
            } else if let Some(rest) = line.strip_prefix("MemAvailable:") {
                snapshot.available_mb = rest
                    .split_whitespace()
                    .next()
                    .and_then(|kb| kb.parse::<u64>().ok())
                    .map(|kb| kb / 1024)
                    .unwrap_or(0);
            }
        }
    }
    snapshot
}

pub fn broker_memory_pressure_at(snapshot: MemorySnapshot) -> BrokerMemoryPressure {
    if snapshot.rss_mb == 0 {
        return BrokerMemoryPressure::Normal;
    }
    if snapshot.total_mb == 0 {
        return if snapshot.rss_mb >= 16_000 {
            BrokerMemoryPressure::PauseBroadFetches
        } else if snapshot.rss_mb >= 12_000 {
            BrokerMemoryPressure::Reduced
        } else {
            BrokerMemoryPressure::Normal
        };
    }

    let reduced_rss = (snapshot.total_mb.saturating_mul(38) / 100).max(8_000);
    let pause_rss = (snapshot.total_mb.saturating_mul(48) / 100).max(reduced_rss + 1);
    let reduced_available = snapshot.total_mb.saturating_mul(45) / 100;
    let pause_available = snapshot.total_mb.saturating_mul(33) / 100;

    if snapshot.rss_mb >= pause_rss
        || (snapshot.available_mb > 0 && snapshot.available_mb <= pause_available)
    {
        BrokerMemoryPressure::PauseBroadFetches
    } else if snapshot.rss_mb >= reduced_rss
        || (snapshot.available_mb > 0 && snapshot.available_mb <= reduced_available)
    {
        BrokerMemoryPressure::Reduced
    } else {
        BrokerMemoryPressure::Normal
    }
}

pub fn current_broker_memory_pressure() -> BrokerMemoryPressure {
    broker_memory_pressure_at(current_memory_snapshot())
}

/// Wait for broad background fetch headroom before starting another expensive
/// response/parse/cache-write burst. Returns false after a bounded wait; callers
/// should settle the queued fetch as unsuccessful so UI pending sets do not wedge.
pub async fn wait_for_broad_fetch_memory_headroom(
    source: &str,
    broker_msg_tx: &tokio::sync::mpsc::UnboundedSender<BrokerMsg>,
) -> bool {
    let start = tokio::time::Instant::now();
    let mut warned = false;
    loop {
        let snapshot = current_memory_snapshot();
        if broker_memory_pressure_at(snapshot) != BrokerMemoryPressure::PauseBroadFetches {
            return true;
        }
        if !warned {
            let _ = broker_msg_tx.send(BrokerMsg::OrderResult(format!(
                "{} broad sync paused for memory headroom (rss={}MB available={}MB total={}MB)",
                source, snapshot.rss_mb, snapshot.available_mb, snapshot.total_mb
            )));
            warned = true;
        }
        if start.elapsed() >= MAX_HEADROOM_WAIT {
            let snapshot = current_memory_snapshot();
            let _ = broker_msg_tx.send(BrokerMsg::OrderResult(format!(
                "{} broad sync skipped one fetch after waiting for memory headroom (rss={}MB available={}MB total={}MB)",
                source, snapshot.rss_mb, snapshot.available_mb, snapshot.total_mb
            )));
            return false;
        }
        tokio::time::sleep(CHECK_INTERVAL).await;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn broker_memory_pressure_uses_rss_and_available_headroom() {
        assert_eq!(
            broker_memory_pressure_at(MemorySnapshot {
                rss_mb: 11_500,
                total_mb: 32_000,
                available_mb: 20_000,
            }),
            BrokerMemoryPressure::Normal
        );
        assert_eq!(
            broker_memory_pressure_at(MemorySnapshot {
                rss_mb: 12_500,
                total_mb: 32_000,
                available_mb: 20_000,
            }),
            BrokerMemoryPressure::Reduced
        );
        assert_eq!(
            broker_memory_pressure_at(MemorySnapshot {
                rss_mb: 15_500,
                total_mb: 32_000,
                available_mb: 20_000,
            }),
            BrokerMemoryPressure::PauseBroadFetches
        );
        assert_eq!(
            broker_memory_pressure_at(MemorySnapshot {
                rss_mb: 8_000,
                total_mb: 32_000,
                available_mb: 10_000,
            }),
            BrokerMemoryPressure::PauseBroadFetches
        );
    }

    #[test]
    fn broker_memory_pressure_fallback_without_meminfo_is_bounded() {
        assert_eq!(
            broker_memory_pressure_at(MemorySnapshot {
                rss_mb: 11_999,
                total_mb: 0,
                available_mb: 0,
            }),
            BrokerMemoryPressure::Normal
        );
        assert_eq!(
            broker_memory_pressure_at(MemorySnapshot {
                rss_mb: 12_000,
                total_mb: 0,
                available_mb: 0,
            }),
            BrokerMemoryPressure::Reduced
        );
        assert_eq!(
            broker_memory_pressure_at(MemorySnapshot {
                rss_mb: 16_000,
                total_mb: 0,
                available_mb: 0,
            }),
            BrokerMemoryPressure::PauseBroadFetches
        );
    }
}
