//! Kraken private REST counter-based rate limiter.
//!
//! Kraken's private REST endpoints share a per-account counter that
//! increases by the per-endpoint cost on each call and decays at a fixed
//! rate (0.5/sec for Intermediate tier). The counter caps at 20; exceeding
//! it triggers an `EAPI:Rate limit exceeded` response. This limiter keeps
//! the counter under that ceiling proactively, then back-offs when Kraken
//! does return a rate-limit error — see `record_rate_limited`.

use std::time::{Duration, Instant};
use tokio::sync::Mutex;

pub(super) const KRAKEN_PRIVATE_REST_MAX_COUNTER: f64 = 20.0;
pub(super) const KRAKEN_PRIVATE_REST_DECAY_PER_SEC: f64 = 0.5;
pub(super) const KRAKEN_PRIVATE_REST_BASE_COOLDOWN: Duration = Duration::from_secs(5);
pub(super) const KRAKEN_PRIVATE_REST_MAX_COOLDOWN: Duration = Duration::from_secs(60);
pub(super) const KRAKEN_PRIVATE_REST_MAX_ATTEMPTS: usize = 3;

#[derive(Debug)]
pub(super) struct KrakenPrivateRestLimiter {
    state: Mutex<KrakenPrivateRestState>,
}

#[derive(Debug)]
struct KrakenPrivateRestState {
    counter: f64,
    last_decay: Instant,
    cooldown_until: Option<Instant>,
    cooldown: Duration,
}

impl KrakenPrivateRestLimiter {
    pub(super) fn new() -> Self {
        Self {
            state: Mutex::new(KrakenPrivateRestState {
                counter: 0.0,
                last_decay: Instant::now(),
                cooldown_until: None,
                cooldown: Duration::ZERO,
            }),
        }
    }

    pub(super) async fn wait(&self, cost: f64) {
        if cost <= 0.0 {
            return;
        }

        loop {
            let wait = {
                let now = Instant::now();
                let mut state = self.state.lock().await;
                state.decay(now);

                let cooldown_wait = if let Some(cooldown_until) = state.cooldown_until {
                    if cooldown_until > now {
                        Some(cooldown_until.saturating_duration_since(now))
                    } else {
                        state.cooldown_until = None;
                        state.cooldown = Duration::ZERO;
                        None
                    }
                } else {
                    None
                };

                if let Some(wait) = cooldown_wait {
                    Some(wait)
                } else if state.counter + cost <= KRAKEN_PRIVATE_REST_MAX_COUNTER {
                    state.counter += cost;
                    None
                } else {
                    let excess = (state.counter + cost) - KRAKEN_PRIVATE_REST_MAX_COUNTER;
                    Some(Duration::from_secs_f64(
                        (excess / KRAKEN_PRIVATE_REST_DECAY_PER_SEC).max(0.25),
                    ))
                }
            };

            if let Some(wait) = wait {
                if !wait.is_zero() {
                    tokio::time::sleep(wait).await;
                }
                continue;
            }
            return;
        }
    }

    pub(super) async fn record_rate_limited(&self, message: &str) -> Duration {
        let explicit_wait = crate::core::kraken::kraken_throttled_wait(message).map(|wait| {
            if wait.is_zero() {
                KRAKEN_PRIVATE_REST_BASE_COOLDOWN
            } else {
                wait.min(KRAKEN_PRIVATE_REST_MAX_COOLDOWN)
            }
        });
        let now = Instant::now();
        let mut state = self.state.lock().await;
        state.decay(now);
        let wait = if let Some(wait) = explicit_wait {
            wait
        } else if state.cooldown_until.is_some_and(|until| until > now) {
            state
                .cooldown
                .max(KRAKEN_PRIVATE_REST_BASE_COOLDOWN)
                .saturating_mul(2)
                .min(KRAKEN_PRIVATE_REST_MAX_COOLDOWN)
        } else {
            KRAKEN_PRIVATE_REST_BASE_COOLDOWN
        };
        state.cooldown = wait;
        let cooldown_until = now + wait;
        state.cooldown_until = Some(
            state
                .cooldown_until
                .map(|existing| existing.max(cooldown_until))
                .unwrap_or(cooldown_until),
        );
        wait
    }

    pub(super) async fn record_success(&self) {
        let now = Instant::now();
        let mut state = self.state.lock().await;
        state.decay(now);
        if state
            .cooldown_until
            .is_some_and(|cooldown_until| cooldown_until <= now)
        {
            state.cooldown_until = None;
            state.cooldown = Duration::ZERO;
        }
    }
}

impl KrakenPrivateRestState {
    fn decay(&mut self, now: Instant) {
        let elapsed = now.saturating_duration_since(self.last_decay);
        if !elapsed.is_zero() {
            self.counter =
                (self.counter - elapsed.as_secs_f64() * KRAKEN_PRIVATE_REST_DECAY_PER_SEC).max(0.0);
            self.last_decay = now;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    #[tokio::test]
    async fn wait_does_not_block_under_counter_ceiling() {
        let limiter = KrakenPrivateRestLimiter::new();
        let start = Instant::now();
        limiter.wait(1.0).await;
        assert!(start.elapsed() < Duration::from_millis(100));
    }

    #[tokio::test]
    async fn record_rate_limited_sets_cooldown_floor() {
        let limiter = KrakenPrivateRestLimiter::new();
        let wait = limiter
            .record_rate_limited("EAPI:Rate limit exceeded")
            .await;
        assert!(wait >= KRAKEN_PRIVATE_REST_BASE_COOLDOWN);
        assert!(wait <= KRAKEN_PRIVATE_REST_MAX_COOLDOWN);
    }

    #[tokio::test]
    async fn record_rate_limited_doubles_on_repeat_until_cap() {
        let limiter = KrakenPrivateRestLimiter::new();
        let first = limiter
            .record_rate_limited("EAPI:Rate limit exceeded")
            .await;
        let second = limiter
            .record_rate_limited("EAPI:Rate limit exceeded")
            .await;
        assert!(second >= first, "second backoff should not shrink");
        assert!(second <= KRAKEN_PRIVATE_REST_MAX_COOLDOWN);
    }
}
