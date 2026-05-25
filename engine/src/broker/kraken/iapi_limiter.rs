//! Process-wide token-bucket limiter for `iapi.kraken.com`.
//!
//! Kraken Pro's internal API sits behind Cloudflare, which rate-limits by
//! client IP. Hammering any one endpoint extends the IP-level ban on the
//! others, so a single limiter must gate the equity ticker, history, and
//! catalog calls together.
//!
//! Design choices forced by the failure mode we were seeing in production:
//!   * **Sustained rate over burst tolerance.** A 260 ms inter-call spacer
//!     (3.8 req/s) triggered a Cloudflare 1015 ban every 2–3 minutes. The
//!     observed ceiling is closer to 0.5–1 req/s averaged over ~10 min,
//!     so the bucket refills at 0.67 tokens/s (one call per ~1.5 s) with a
//!     small capacity for interactive bursts.
//!   * **Exponential backoff escalation.** A flat 600 s window after each
//!     1015 left us tripping the ban immediately on every reopen because
//!     Cloudflare's tracking memory is longer than 600 s. We now double the
//!     window on consecutive arms (within `escalation_reset_after`), capped
//!     at `max_backoff_secs`, and reset the counter only after a quiet run.
//!   * **Drain tokens on 429.** The bucket starts at zero immediately after
//!     a ban, so the first wave of post-cooldown traffic ramps in at the
//!     refill rate rather than firing 8 calls instantly.
//!   * **Persistence across restarts.** Otherwise a crash mid-ban gives the
//!     next launch a free pass to immediately re-trip Cloudflare.

use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, AtomicI64, Ordering};
use std::sync::{Mutex as StdMutex, OnceLock};
use std::time::{Duration, Instant};
use tokio::sync::Mutex as TokioMutex;

/// Tunables for the iapi limiter. Defaults are calibrated for the observed
/// Cloudflare ceiling on `iapi.kraken.com`.
#[derive(Debug, Clone)]
pub struct IapiLimiterConfig {
    /// Maximum tokens in the bucket. Each iapi call costs `cost` tokens
    /// (history = 1.0, ticker = 1.0, catalog page = 2.0).
    pub capacity: f64,
    /// Refill rate, tokens per second. Steady-state request rate.
    pub refill_per_sec: f64,
    /// Backoff seconds applied to a plain HTTP 429 body.
    pub default_backoff_secs: i64,
    /// Backoff seconds applied when the body matches Cloudflare 1015
    /// (IP-level rate limit, the slower-to-clear case).
    pub cloudflare_backoff_secs: i64,
    /// Hard ceiling on backoff, regardless of escalation count.
    pub max_backoff_secs: i64,
    /// If a fresh 429 lands within this window after a prior arm, the
    /// escalation counter increments and the backoff doubles. Beyond this
    /// window, the next arm starts the count fresh.
    pub escalation_reset_after: Duration,
    /// Where to mirror cooldown state so a restart respects an in-flight ban.
    /// `None` disables persistence (used in tests and unauthenticated tools).
    pub persistence_path: Option<PathBuf>,
}

impl Default for IapiLimiterConfig {
    fn default() -> Self {
        Self {
            capacity: 8.0,
            refill_per_sec: 0.67,
            default_backoff_secs: 90,
            cloudflare_backoff_secs: 600,
            max_backoff_secs: 3600,
            escalation_reset_after: Duration::from_secs(600),
            persistence_path: None,
        }
    }
}

#[derive(Debug)]
pub struct IapiLimiter {
    config: IapiLimiterConfig,
    /// Sync-readable cooldown deadline (Unix seconds). Read by hot-path probes
    /// like `iapi_rate_limited_for_secs()` without acquiring the bucket lock.
    cooldown_until_unix: AtomicI64,
    bucket: TokioMutex<TokenBucket>,
    escalation: StdMutex<EscalationState>,
}

#[derive(Debug)]
struct TokenBucket {
    tokens: f64,
    last_refill: Instant,
}

#[derive(Debug)]
struct EscalationState {
    consecutive: u32,
    last_arm: Option<Instant>,
}

#[derive(Debug, serde::Serialize, serde::Deserialize, Default)]
struct PersistedState {
    cooldown_until_unix: i64,
    consecutive: u32,
    last_arm_unix: i64,
}

/// Process-wide instance. Initialised lazily with `IapiLimiterConfig::default`
/// if `iapi_limiter_init` is not called first.
static INSTANCE: OnceLock<IapiLimiter> = OnceLock::new();

/// Latches on the first successful iapi response so the diagnostic
/// header dump fires once at INFO (visible without enabling debug
/// logging) and subsequent responses log at DEBUG. Helps callers see
/// at-a-glance whether iapi exposes a budget header without flooding
/// the log on every fetch.
static IAPI_HEADERS_LOGGED_ONCE: AtomicBool = AtomicBool::new(false);

/// Header names worth surfacing for rate-limit diagnosis. Cloudflare and
/// Kraken sometimes return upstream RateLimit-* headers (RFC 9239 draft
/// names) and Cloudflare always sets `cf-ray`. If none of these appear
/// on a 200 OK, we know we'll have to estimate the ceiling empirically.
const RATE_LIMIT_HEADER_NAMES: &[&str] = &[
    "x-ratelimit-limit",
    "x-ratelimit-remaining",
    "x-ratelimit-reset",
    "ratelimit-limit",
    "ratelimit-remaining",
    "ratelimit-reset",
    "ratelimit-policy",
    "retry-after",
    "cf-ray",
    "cf-cache-status",
    "server",
];

/// Log a one-line summary of any rate-limit-related response headers
/// returned by an iapi endpoint after a 2xx response. First call per
/// process logs at INFO; subsequent calls at DEBUG. Use the `endpoint`
/// label to differentiate sites (e.g. "ticker", "history", "catalog").
pub fn log_iapi_response_headers(headers: &reqwest::header::HeaderMap, endpoint: &str) {
    let mut found: Vec<String> = Vec::new();
    for name in RATE_LIMIT_HEADER_NAMES {
        if let Some(value) = headers.get(*name) {
            if let Ok(s) = value.to_str() {
                found.push(format!("{name}={s}"));
            }
        }
    }
    let summary = if found.is_empty() {
        "(none of the inspected headers were present)".to_string()
    } else {
        found.join(" ")
    };
    if !IAPI_HEADERS_LOGGED_ONCE.swap(true, Ordering::Relaxed) {
        tracing::info!("iapi {endpoint} response headers (one-shot diagnostic): {summary}");
    } else {
        tracing::debug!("iapi {endpoint} response headers: {summary}");
    }
}

/// Access the process-wide limiter. Lazy-creates with defaults if uninit.
pub fn iapi_limiter() -> &'static IapiLimiter {
    INSTANCE.get_or_init(|| IapiLimiter::new(IapiLimiterConfig::default()))
}

/// Install a configured limiter early in startup (typically from the app
/// crate, which knows the persistence path). Subsequent `iapi_limiter()`
/// calls return this instance. Returns `Err` if a limiter was already
/// initialised (lazy default already won the race).
pub fn iapi_limiter_init(config: IapiLimiterConfig) -> Result<(), &'static str> {
    INSTANCE
        .set(IapiLimiter::new(config))
        .map_err(|_| "iapi limiter already initialised")
}

/// Acquire result returned by [`IapiLimiter::acquire`]. The `Err` carries the
/// remaining cooldown seconds so callers can surface a stable user-facing
/// message instead of guessing.
pub type AcquireResult = Result<(), i64>;

impl IapiLimiter {
    pub fn new(config: IapiLimiterConfig) -> Self {
        let persisted = config
            .persistence_path
            .as_ref()
            .and_then(|p| load_persisted(p));
        let now_unix = chrono::Utc::now().timestamp();
        let (cooldown_until, escalation) = match persisted {
            Some(state) if state.cooldown_until_unix > now_unix => {
                // Reconstruct an Instant for the persisted `last_arm` so the
                // escalation window keeps decaying after a restart.
                let last_arm = if state.last_arm_unix > 0 && state.last_arm_unix <= now_unix {
                    let delta = (now_unix - state.last_arm_unix) as u64;
                    Instant::now().checked_sub(Duration::from_secs(delta))
                } else {
                    None
                };
                (
                    state.cooldown_until_unix,
                    EscalationState {
                        consecutive: state.consecutive,
                        last_arm,
                    },
                )
            }
            _ => (0, EscalationState::default()),
        };
        let starting_tokens = if cooldown_until > now_unix {
            // Already in cooldown — start drained so resume traffic ramps.
            0.0
        } else {
            config.capacity
        };
        Self {
            cooldown_until_unix: AtomicI64::new(cooldown_until),
            bucket: TokioMutex::new(TokenBucket {
                tokens: starting_tokens,
                last_refill: Instant::now(),
            }),
            escalation: StdMutex::new(escalation),
            config,
        }
    }

    /// Remaining cooldown in seconds, or `None` if free to call. Cheap; safe
    /// to call from any thread without awaiting.
    pub fn remaining_backoff_secs(&self) -> Option<i64> {
        let until = self.cooldown_until_unix.load(Ordering::Relaxed);
        let now = chrono::Utc::now().timestamp();
        if until > now { Some(until - now) } else { None }
    }

    /// Wait for `cost` tokens. Returns immediately with `Err(secs)` if the
    /// limiter is in cooldown; otherwise sleeps until enough refill has
    /// accumulated and deducts the cost.
    pub async fn acquire(&self, cost: f64) -> AcquireResult {
        if let Some(remaining) = self.remaining_backoff_secs() {
            return Err(remaining);
        }
        if cost <= 0.0 {
            return Ok(());
        }
        loop {
            let wait = {
                let mut bucket = self.bucket.lock().await;
                bucket.refill(self.config.refill_per_sec, self.config.capacity, Instant::now());
                if bucket.tokens >= cost {
                    bucket.tokens -= cost;
                    None
                } else {
                    let deficit = cost - bucket.tokens;
                    Some(Duration::from_secs_f64(
                        (deficit / self.config.refill_per_sec).max(0.05),
                    ))
                }
            };
            // A cooldown may have been armed while we were waiting on the
            // bucket — re-check before sleeping or returning success.
            if let Some(remaining) = self.remaining_backoff_secs() {
                return Err(remaining);
            }
            match wait {
                Some(d) => {
                    tokio::time::sleep(d).await;
                    continue;
                }
                None => return Ok(()),
            }
        }
    }

    /// Arm the cooldown after seeing a 429. Body is inspected for Cloudflare
    /// 1015 to pick the slower window. Concurrent racers all run this and
    /// CAS toward the latest expiry, so the longest backoff wins. Returns
    /// the chosen backoff in seconds (post-cap), useful for the caller's
    /// log line.
    pub async fn record_rate_limited(&self, body: &str) -> i64 {
        let is_cf = body.contains("1015");
        let base = if is_cf {
            self.config.cloudflare_backoff_secs
        } else {
            self.config.default_backoff_secs
        };
        let now = Instant::now();
        let (chosen_secs, consecutive) = {
            let mut esc = self.escalation.lock().expect("escalation mutex poisoned");
            let consecutive = if esc
                .last_arm
                .map(|t| now.saturating_duration_since(t) < self.config.escalation_reset_after)
                .unwrap_or(false)
            {
                esc.consecutive.saturating_add(1)
            } else {
                1
            };
            esc.consecutive = consecutive;
            esc.last_arm = Some(now);
            // Exponential: base * 2^(consecutive-1). Clamp the shift count
            // before computing the multiplier so we don't overflow when the
            // counter runs away (a misbehaving server could spam 429s).
            let shift = consecutive.saturating_sub(1).min(20);
            let mult: u64 = 1u64 << shift;
            let secs = (base as u64).saturating_mul(mult);
            let capped = (secs as i64).min(self.config.max_backoff_secs);
            (capped, consecutive)
        };
        let now_unix = chrono::Utc::now().timestamp();
        let until = now_unix.saturating_add(chosen_secs);
        // CAS-bump cooldown_until — never shrink an active longer window.
        let mut current = self.cooldown_until_unix.load(Ordering::Relaxed);
        while until > current {
            match self.cooldown_until_unix.compare_exchange_weak(
                current,
                until,
                Ordering::Relaxed,
                Ordering::Relaxed,
            ) {
                Ok(_) => break,
                Err(actual) => current = actual,
            }
        }
        // Drain tokens. Post-cooldown the bucket refills from zero, so the
        // first calls trickle through at the refill rate instead of firing
        // a burst that re-trips Cloudflare immediately.
        {
            let mut bucket = self.bucket.lock().await;
            bucket.tokens = 0.0;
            bucket.last_refill = Instant::now();
        }
        self.persist(
            self.cooldown_until_unix.load(Ordering::Relaxed),
            consecutive,
            now_unix,
        );
        let final_until = self.cooldown_until_unix.load(Ordering::Relaxed);
        (final_until - now_unix).max(0)
    }

    /// Note a successful call. If the last arm is far enough behind us,
    /// reset the escalation counter so a future 429 starts at the base
    /// window instead of inheriting the doubled history.
    pub async fn record_success(&self) {
        let now = Instant::now();
        let mut esc = self.escalation.lock().expect("escalation mutex poisoned");
        if let Some(last_arm) = esc.last_arm {
            if now.saturating_duration_since(last_arm) > self.config.escalation_reset_after {
                esc.consecutive = 0;
                esc.last_arm = None;
            }
        }
    }

    fn persist(&self, until_unix: i64, consecutive: u32, last_arm_unix: i64) {
        let Some(path) = self.config.persistence_path.clone() else {
            return;
        };
        let state = PersistedState {
            cooldown_until_unix: until_unix,
            consecutive,
            last_arm_unix,
        };
        if let Ok(json) = serde_json::to_string(&state) {
            // Best-effort write. A failure here only costs us correct
            // cross-restart behaviour for this single update; in-memory
            // state still tracks the live ban.
            let _ = std::fs::write(&path, json);
        }
    }
}

impl Default for EscalationState {
    fn default() -> Self {
        Self {
            consecutive: 0,
            last_arm: None,
        }
    }
}

impl TokenBucket {
    fn refill(&mut self, rate: f64, capacity: f64, now: Instant) {
        let elapsed = now.saturating_duration_since(self.last_refill).as_secs_f64();
        if elapsed > 0.0 {
            self.tokens = (self.tokens + elapsed * rate).min(capacity);
            self.last_refill = now;
        }
    }
}

fn load_persisted(path: &std::path::Path) -> Option<PersistedState> {
    let body = std::fs::read_to_string(path).ok()?;
    serde_json::from_str(&body).ok()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration as StdDuration;
    use tokio::time::Duration as TokioDuration;

    fn fast_config() -> IapiLimiterConfig {
        // Faster refill so tests don't take seconds.
        IapiLimiterConfig {
            capacity: 4.0,
            refill_per_sec: 40.0, // one token per 25 ms
            default_backoff_secs: 90,
            cloudflare_backoff_secs: 600,
            max_backoff_secs: 3600,
            escalation_reset_after: StdDuration::from_secs(60),
            persistence_path: None,
        }
    }

    #[tokio::test]
    async fn acquire_succeeds_within_capacity() {
        let lim = IapiLimiter::new(fast_config());
        for _ in 0..4 {
            lim.acquire(1.0).await.expect("free");
        }
    }

    #[tokio::test]
    async fn acquire_waits_when_bucket_empty() {
        let lim = IapiLimiter::new(fast_config());
        for _ in 0..4 {
            lim.acquire(1.0).await.expect("free");
        }
        let start = std::time::Instant::now();
        lim.acquire(1.0).await.expect("eventually free");
        let elapsed = start.elapsed();
        // Refill is 40 tok/s → one token in 25 ms. Allow generous lower bound.
        assert!(
            elapsed >= TokioDuration::from_millis(15),
            "should have waited, got {elapsed:?}"
        );
    }

    #[tokio::test]
    async fn acquire_short_circuits_during_cooldown() {
        let lim = IapiLimiter::new(fast_config());
        lim.record_rate_limited("Too Many Requests").await;
        let err = lim.acquire(1.0).await.unwrap_err();
        assert!(err > 0, "expected positive remaining backoff, got {err}");
    }

    #[tokio::test]
    async fn record_rate_limited_arms_default_window() {
        let lim = IapiLimiter::new(fast_config());
        let secs = lim.record_rate_limited("Too Many Requests").await;
        assert!(secs >= 89 && secs <= 90, "got {secs}");
    }

    #[tokio::test]
    async fn cloudflare_1015_uses_longer_window() {
        let lim = IapiLimiter::new(fast_config());
        let secs = lim.record_rate_limited("<html>error code: 1015</html>").await;
        assert!(secs >= 599 && secs <= 600, "got {secs}");
    }

    #[tokio::test]
    async fn consecutive_arms_escalate_within_window() {
        let lim = IapiLimiter::new(fast_config());
        let first = lim.record_rate_limited("Too Many Requests").await;
        let second = lim.record_rate_limited("Too Many Requests").await;
        let third = lim.record_rate_limited("Too Many Requests").await;
        // first=90, second>=180, third>=360 (allowing for the clock-tick
        // difference between arm time and the read on the line below).
        assert!(second >= first * 2 - 2, "first={first} second={second}");
        assert!(third >= second * 2 - 2, "second={second} third={third}");
    }

    #[tokio::test]
    async fn escalation_capped_at_max() {
        let mut cfg = fast_config();
        cfg.max_backoff_secs = 500;
        let lim = IapiLimiter::new(cfg);
        for _ in 0..30 {
            lim.record_rate_limited("Too Many Requests").await;
        }
        let remaining = lim.remaining_backoff_secs().unwrap();
        assert!(remaining <= 500, "got {remaining}");
    }

    #[tokio::test]
    async fn record_rate_limited_drains_bucket() {
        let lim = IapiLimiter::new(fast_config());
        // Bucket starts full at capacity 4.0.
        lim.record_rate_limited("Too Many Requests").await;
        let bucket = lim.bucket.lock().await;
        assert!(bucket.tokens < 0.01, "tokens={}", bucket.tokens);
    }

    #[tokio::test]
    async fn cas_keeps_longest_window_when_short_arm_lands_late() {
        let lim = IapiLimiter::new(fast_config());
        // Arm the long Cloudflare window first.
        let cf = lim.record_rate_limited("error code: 1015").await;
        assert!(cf >= 599, "{cf}");
        // A subsequent plain 429 escalates from consecutive=2 → 180s; that
        // is still smaller than the live 600s window, so the global expiry
        // must NOT shrink.
        let _ = lim.record_rate_limited("Too Many Requests").await;
        let still_long = lim.remaining_backoff_secs().unwrap();
        // Note: escalation pushed default to 180s but cooldown_until stays
        // at the larger CF deadline.
        assert!(still_long >= 590, "got {still_long}");
    }

    #[test]
    fn persistence_round_trip() {
        let tmp = tempfile::NamedTempFile::new().expect("tempfile");
        let state = PersistedState {
            cooldown_until_unix: 1_700_000_000,
            consecutive: 3,
            last_arm_unix: 1_699_999_000,
        };
        std::fs::write(tmp.path(), serde_json::to_string(&state).unwrap()).unwrap();
        let loaded = load_persisted(tmp.path()).expect("decoded");
        assert_eq!(loaded.cooldown_until_unix, 1_700_000_000);
        assert_eq!(loaded.consecutive, 3);
        assert_eq!(loaded.last_arm_unix, 1_699_999_000);
    }

    #[tokio::test]
    async fn restore_in_flight_cooldown_blocks_acquire() {
        let tmp = tempfile::NamedTempFile::new().expect("tempfile");
        let now_unix = chrono::Utc::now().timestamp();
        let state = PersistedState {
            cooldown_until_unix: now_unix + 300,
            consecutive: 2,
            last_arm_unix: now_unix - 5,
        };
        std::fs::write(tmp.path(), serde_json::to_string(&state).unwrap()).unwrap();
        let cfg = IapiLimiterConfig {
            persistence_path: Some(tmp.path().to_path_buf()),
            ..fast_config()
        };
        let lim = IapiLimiter::new(cfg);
        assert!(lim.remaining_backoff_secs().is_some());
        let err = lim.acquire(1.0).await.unwrap_err();
        assert!(err > 0);
    }

    #[tokio::test]
    async fn restore_skips_expired_cooldown() {
        let tmp = tempfile::NamedTempFile::new().expect("tempfile");
        let now_unix = chrono::Utc::now().timestamp();
        let state = PersistedState {
            cooldown_until_unix: now_unix - 60,
            consecutive: 1,
            last_arm_unix: now_unix - 100,
        };
        std::fs::write(tmp.path(), serde_json::to_string(&state).unwrap()).unwrap();
        let cfg = IapiLimiterConfig {
            persistence_path: Some(tmp.path().to_path_buf()),
            ..fast_config()
        };
        let lim = IapiLimiter::new(cfg);
        assert!(lim.remaining_backoff_secs().is_none());
        lim.acquire(1.0).await.expect("free");
    }
}
