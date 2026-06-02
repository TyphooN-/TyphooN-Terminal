//! Process-wide token-bucket limiter for `iapi.kraken.com`.
//!
//! Kraken Pro's internal API sits behind Cloudflare, which rate-limits by
//! client IP. Hammering any one endpoint extends the IP-level ban on the
//! others, so a single limiter must gate the equity ticker, history, and
//! catalog calls together.
//!
//! Design choices forced by the failure mode we were seeing in production:
//!   * **Sustained rate over burst tolerance.** A 260 ms inter-call spacer
//!     (3.8 req/s) previously triggered a Cloudflare 1015 ban every 2–3
//!     minutes on one route/IP mix, but later clean sessions have reached the
//!     former 5 req/s cap. Start conservatively, then let AIMD discover the
//!     current ceiling instead of baking one historical observation into the
//!     scheduler.
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
/// Cloudflare ceiling on `iapi.kraken.com`. AIMD parameters auto-discover
/// the actual ceiling so the user doesn't have to guess.
#[derive(Debug, Clone)]
pub struct IapiLimiterConfig {
    /// Maximum tokens in the bucket. Each iapi call costs `cost` tokens
    /// (history = 1.0, ticker = 1.0, catalog page = 2.0).
    pub capacity: f64,
    /// Initial refill rate, tokens per second. When AIMD is enabled (the
    /// default) this is just the starting point — the live rate adapts
    /// upward during clean traffic and downward on 429s. When AIMD is
    /// disabled, this is the static steady-state rate forever.
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
    /// AIMD (Additive Increase / Multiplicative Decrease) rate adaptation.
    /// When enabled, the live refill rate climbs during clean runs and
    /// halves on each 429, converging on Cloudflare's actual ceiling
    /// without manual tuning. Set false to pin the rate at
    /// `refill_per_sec` forever (useful in tests).
    pub aimd_enabled: bool,
    /// Floor for the AIMD rate — even after repeated 429s the limiter
    /// will not drop below this. Default 0.5 req/s: aggressive enough to
    /// catch up on a small backlog, conservative enough that Cloudflare
    /// shouldn't 1015 us at this rate.
    pub aimd_min_rate: f64,
    /// Ceiling for the AIMD rate. The limiter will not climb above this even
    /// on a long clean run. Default 20.0 req/s: high enough that a clean
    /// session can probe beyond the old 5 req/s cap, still bounded so a bad
    /// config cannot become raw uncapped hammering.
    pub aimd_max_rate: f64,
    /// How often (during clean traffic) to tighten request pacing by one
    /// `aimd_resolution` interval step. The legacy field name is rate-shaped,
    /// but iapi tuning is interval-precise: at rate `r`, the next clean step is
    /// `1 / ((1 / r) - aimd_resolution)`, clamped by `aimd_max_rate`.
    pub aimd_increase_interval: Duration,
    /// Legacy additive increment retained for config/test compatibility. The
    /// iapi limiter now derives the additive step from `aimd_resolution` as a
    /// request-interval delta instead of adding this value directly to req/s.
    pub aimd_increment_per_step: f64,
    /// Multiplicative decrease applied to the rate on each 429. Default
    /// 0.5 (TCP-style halving) — aggressive enough to back off hard
    /// when Cloudflare pushes back, gentle enough that occasional false
    /// positives don't cripple throughput.
    pub aimd_decrease_factor: f64,
    /// Precision used when deriving the pacing ceiling after a rate-limit hit,
    /// expressed in seconds of request spacing. If the current interval just
    /// failed and no adjacent successful rate was observed, future ramps cap at
    /// `failed_interval + aimd_resolution`. Default 0.01 means hundredth-second
    /// pacing discovery, which is the desired iapi precision.
    pub aimd_resolution: f64,
    /// How long the rate must hold steady (no increase, no 429) before
    /// the limiter declares it "converged" and starts persisting the
    /// value. Once converged the rate is mirrored to disk so the next
    /// run skips the ramp-up phase entirely. A subsequent 429 unsets
    /// the converged flag — Cloudflare can change limits, so we keep
    /// AIMD active as a safety net even after convergence.
    pub aimd_tuned_after: Duration,
    /// Legacy headroom fallback used only if interval-resolution math produces
    /// an invalid candidate. Normal iapi ceiling discovery should use
    /// `last_successful_rate` or `failed_interval + aimd_resolution`, not this
    /// coarse percentage rule.
    pub aimd_ceiling_headroom: f64,
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
            aimd_enabled: true,
            aimd_min_rate: 0.5,
            aimd_max_rate: 20.0,
            aimd_increase_interval: Duration::from_secs(10),
            aimd_increment_per_step: 0.01,
            aimd_decrease_factor: 0.5,
            aimd_resolution: 0.01,
            aimd_tuned_after: Duration::from_secs(30 * 60),
            aimd_ceiling_headroom: 0.95,
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
    /// Live AIMD-controlled refill rate. Initialised from
    /// `config.refill_per_sec`; mutated by the AIMD ramp / decrease
    /// paths inside `acquire` and `record_rate_limited`.
    current_rate: f64,
    /// Wall-clock anchor for the next AIMD pacing step. Reset on each
    /// successful interval-tightening bump and on each 429-triggered decrease
    /// so the cadence restarts cleanly from each rate-change event.
    last_rate_change: Instant,
    /// `true` once `last_rate_change` has been older than
    /// `aimd_tuned_after`. Means we've found a stable ceiling and the
    /// value has been persisted. Cleared on any 429 so a future
    /// Cloudflare tightening re-enters the ramp/decrease cycle.
    converged: bool,
    /// ssthresh-style observed ceiling: the highest rate Cloudflare tolerated,
    /// or one configured interval-resolution step slower than the failed rate.
    /// `None` until the first 429 — until then ramps can go all the way to
    /// `aimd_max_rate`. Once set, the ramp caps at this value so the limiter
    /// converges just below the real ceiling instead of oscillating around it.
    discovered_ceiling: Option<f64>,
    /// Highest rate that produced a successful iapi response in this session.
    /// Used to turn a following 429 into a precise empirical ceiling instead
    /// of blindly applying coarse percentage headroom.
    last_successful_rate: Option<f64>,
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
    /// AIMD-converged rate (req/s) from the last session, or 0.0 when
    /// we never reached convergence. `#[serde(default)]` keeps older
    /// persistence files (pre-AIMD) decoding cleanly — they just won't
    /// carry a tuned value and the new session ramps from scratch.
    #[serde(default)]
    tuned_rate: f64,
    /// Latest non-converged AIMD checkpoint (req/s), written during clean
    /// increases and after 429 decreases so restarts do not throw away the
    /// current learning run. Unlike `tuned_rate`, restoring this keeps AIMD
    /// active instead of marking the limiter converged.
    #[serde(default)]
    checkpoint_rate: f64,
    /// ssthresh observed ceiling from the last session, or 0.0 when no
    /// 429 has been seen yet. Restored on startup so the next ramp
    /// caps at the same value instead of climbing back into the
    /// already-known ban zone.
    #[serde(default)]
    discovered_ceiling: f64,
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
        let (cooldown_until, escalation, restored_rate, restored_converged, persisted_ceiling) =
            match &persisted {
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
                        restored_aimd_rate(state),
                        state.tuned_rate > 0.0,
                        state.discovered_ceiling,
                    )
                }
                Some(state) => {
                    let recent_rate_limit = state.last_arm_unix > 0
                        && state.last_arm_unix <= now_unix
                        && (now_unix - state.last_arm_unix) as u64
                            <= config.escalation_reset_after.as_secs();
                    // Once the persisted cooldown/escalation window is gone, do
                    // not keep a stale empirical ceiling or "converged" latch
                    // forever. Kraken/Cloudflare limits vary by session/IP; an
                    // old 0.66 req/s discovery was freezing future launches at
                    // crawler-idle speed and preventing AIMD from probing back
                    // toward max intensity. Keep the learned rate as the safe
                    // starting point, but let AIMD ramp again.
                    (
                        0,
                        EscalationState::default(),
                        restored_aimd_rate(state),
                        false,
                        if recent_rate_limit {
                            state.discovered_ceiling
                        } else {
                            0.0
                        },
                    )
                }
                None => (0, EscalationState::default(), 0.0, false, 0.0),
            };
        let starting_tokens = if cooldown_until > now_unix {
            // Already in cooldown — start drained so resume traffic ramps.
            0.0
        } else {
            config.capacity
        };
        // Prefer a persisted AIMD rate over the cold config default. A
        // converged tuned rate resumes as converged; a checkpoint resumes at
        // the learned rate but keeps AIMD active so the new process continues
        // probing instead of pretending it found a final ceiling. Clamp into
        // the configured band so a stale/corrupt persisted value cannot drive
        // us out of bounds.
        let raw_starting_rate = if restored_rate > 0.0 {
            restored_rate
        } else {
            config.refill_per_sec
        };
        let starting_rate = raw_starting_rate
            .max(config.aimd_min_rate)
            .min(config.aimd_max_rate);
        let starts_converged = restored_converged;
        let starting_ceiling = if persisted_ceiling > 0.0 {
            Some(
                persisted_ceiling
                    .max(config.aimd_min_rate)
                    .min(config.aimd_max_rate),
            )
        } else {
            None
        };
        if starts_converged {
            tracing::info!(
                "iapi AIMD: restored tuned rate {:.2} req/s from persisted state",
                starting_rate
            );
        } else if restored_rate > 0.0 {
            tracing::info!(
                "iapi AIMD: restored checkpoint rate {:.2} req/s from persisted state — AIMD still active",
                starting_rate
            );
        }
        if let Some(c) = starting_ceiling {
            tracing::info!(
                "iapi AIMD: restored discovered ceiling {:.2} req/s — ramp will cap here",
                c
            );
        }
        Self {
            cooldown_until_unix: AtomicI64::new(cooldown_until),
            bucket: TokioMutex::new(TokenBucket {
                tokens: starting_tokens,
                last_refill: Instant::now(),
                current_rate: starting_rate,
                last_rate_change: Instant::now(),
                converged: starts_converged,
                discovered_ceiling: starting_ceiling,
                last_successful_rate: None,
            }),
            escalation: StdMutex::new(escalation),
            config,
        }
    }

    /// Current live AIMD rate, observed for tests and diagnostics.
    /// Cheap (one mutex lock, no I/O); not exposed in the hot path.
    pub async fn current_rate_per_sec(&self) -> f64 {
        self.bucket.lock().await.current_rate
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
    /// accumulated and deducts the cost. AIMD ramp-up runs here too:
    /// every `aimd_increase_interval` of clean traffic bumps the live
    /// pacing by one `aimd_resolution` interval step, logged at INFO so the
    /// discovered ceiling shows up in tracing.
    pub async fn acquire(&self, cost: f64) -> AcquireResult {
        if let Some(remaining) = self.remaining_backoff_secs() {
            return Err(remaining);
        }
        if cost <= 0.0 {
            return Ok(());
        }
        loop {
            let mut converged_now: Option<f64> = None;
            let mut checkpoint_now: Option<f64> = None;
            let wait = {
                let mut bucket = self.bucket.lock().await;
                let now = Instant::now();
                // AIMD interval-tightening step: if enough time has passed
                // since the last pacing change AND we're below the effective
                // ceiling, shorten request spacing by `aimd_resolution` and
                // convert back to req/s. The effective ceiling is the
                // lesser of `aimd_max_rate` and the empirically discovered
                // ssthresh — if Cloudflare 1015'd us at some rate before,
                // we cap the ramp just below there instead of climbing
                // back into the known ban zone.
                let effective_max = bucket
                    .discovered_ceiling
                    .map(|c| c.min(self.config.aimd_max_rate))
                    .unwrap_or(self.config.aimd_max_rate);
                if self.config.aimd_enabled
                    && !bucket.converged
                    && bucket.current_rate < effective_max
                    && now.saturating_duration_since(bucket.last_rate_change)
                        >= self.config.aimd_increase_interval
                {
                    let new_rate = next_rate_for_shorter_interval(
                        bucket.current_rate,
                        self.config.aimd_resolution,
                        effective_max,
                    );
                    if (new_rate - bucket.current_rate).abs() >= 0.000_001 {
                        tracing::info!(
                            "iapi AIMD: rate ↑ {:.4} → {:.4} req/s after clean run ({:.2}s → {:.2}s pacing)",
                            bucket.current_rate,
                            new_rate,
                            rate_to_interval_secs(bucket.current_rate),
                            rate_to_interval_secs(new_rate)
                        );
                        bucket.current_rate = new_rate;
                        bucket.last_rate_change = now;
                        checkpoint_now = Some(new_rate);
                    }
                }
                // Convergence check: rate has been steady (no AIMD bump,
                // no 429) for `aimd_tuned_after`. Mark converged and
                // remember the value for the post-lock persist call.
                if self.config.aimd_enabled
                    && !bucket.converged
                    && now.saturating_duration_since(bucket.last_rate_change)
                        >= self.config.aimd_tuned_after
                {
                    bucket.converged = true;
                    converged_now = Some(bucket.current_rate);
                    tracing::info!(
                        "iapi AIMD: rate converged at {:.4} req/s ({:.2}s pacing) — persisting and pausing ramp-up",
                        bucket.current_rate,
                        rate_to_interval_secs(bucket.current_rate)
                    );
                }
                let rate = bucket.current_rate.max(0.001);
                bucket.refill(rate, self.config.capacity, now);
                if bucket.tokens >= cost {
                    bucket.tokens -= cost;
                    None
                } else {
                    let deficit = cost - bucket.tokens;
                    Some(Duration::from_secs_f64((deficit / rate).max(0.05)))
                }
            };
            // Persist outside the bucket lock so file IO doesn't block
            // concurrent acquires. Persistence is best-effort — failure
            // just means the next run re-discovers the rate. The
            // escalation guard is read inside an inner block so the
            // std::sync::MutexGuard is provably dropped before any
            // `.await` — required for this future to be `Send`.
            if let Some(tuned_rate) = converged_now {
                self.persist_aimd_state(tuned_rate, 0.0).await;
            } else if let Some(checkpoint_rate) = checkpoint_now {
                self.persist_aimd_state(0.0, checkpoint_rate).await;
            }
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
        // Drain tokens AND apply AIMD multiplicative-decrease so the
        // resumed traffic comes back at a slower rate that Cloudflare
        // is less likely to ban again. The rate floor (`aimd_min_rate`)
        // keeps us from collapsing to zero on repeated bans. Clearing
        // `converged` means the ramp-up restarts; setting (or lowering)
        // `discovered_ceiling` to the last successful pacing level, or one
        // 0.01s interval step slower than the failed level, tells the next ramp
        // where to stop — ssthresh-style without coarse percentage headroom.
        let (post_rate, post_ceiling) = {
            let mut bucket = self.bucket.lock().await;
            bucket.tokens = 0.0;
            bucket.last_refill = Instant::now();
            bucket.converged = false;
            // ssthresh update: the pacing interval that JUST 429'd is now
            // known-bad. Prefer the highest rate that actually produced a
            // successful response in this session; otherwise step one
            // `aimd_resolution` interval slower than the failed pacing. The old
            // percentage-headroom heuristic is only a pathological fallback.
            if self.config.aimd_enabled {
                let failed_rate = bucket.current_rate;
                let precise_candidate = interval_precision_ceiling_candidate(
                    failed_rate,
                    bucket.last_successful_rate,
                    self.config.aimd_resolution,
                );
                let fallback_candidate = failed_rate * self.config.aimd_ceiling_headroom;
                let raw_ceiling = if precise_candidate.is_finite() && precise_candidate > 0.0 {
                    precise_candidate
                } else {
                    fallback_candidate
                };
                let new_ceiling = raw_ceiling
                    .max(self.config.aimd_min_rate)
                    .min(self.config.aimd_max_rate);
                let prev = bucket.discovered_ceiling;
                let merged = match prev {
                    Some(p) => p.min(new_ceiling),
                    None => new_ceiling,
                };
                if prev != Some(merged) {
                    tracing::info!(
                        "iapi AIMD: discovered ceiling = {:.4} req/s ({:.2}s pacing; last safe {:?}, rate {:.4} / {:.2}s just {}'d)",
                        merged,
                        rate_to_interval_secs(merged),
                        bucket.last_successful_rate,
                        failed_rate,
                        rate_to_interval_secs(failed_rate),
                        if is_cf { "1015" } else { "429" }
                    );
                    bucket.discovered_ceiling = Some(merged);
                }
                let new_rate = (bucket.current_rate * self.config.aimd_decrease_factor)
                    .max(self.config.aimd_min_rate);
                if (new_rate - bucket.current_rate).abs() >= 0.001 {
                    tracing::warn!(
                        "iapi AIMD: rate ↓ {:.2} → {:.2} req/s after {}",
                        bucket.current_rate,
                        new_rate,
                        if is_cf { "Cloudflare 1015" } else { "HTTP 429" }
                    );
                    bucket.current_rate = new_rate;
                    bucket.last_rate_change = Instant::now();
                }
            }
            (
                bucket.current_rate,
                bucket.discovered_ceiling.unwrap_or(0.0),
            )
        };
        // Persist the post-429 rate as a checkpoint AND the freshly-updated
        // ceiling so a restart mid-throttle resumes conservatively but keeps
        // AIMD active. `tuned_rate` deliberately stays zero here because the
        // 429 proved the prior rate was not converged.
        self.persist_with_explicit_state(
            self.cooldown_until_unix.load(Ordering::Relaxed),
            consecutive,
            now_unix,
            0.0,
            post_rate,
            post_ceiling,
        );
        let final_until = self.cooldown_until_unix.load(Ordering::Relaxed);
        (final_until - now_unix).max(0)
    }

    /// Note a successful call. If the last arm is far enough behind us,
    /// reset the escalation counter so a future 429 starts at the base
    /// window instead of inheriting the doubled history.
    pub async fn record_success(&self) {
        {
            let mut bucket = self.bucket.lock().await;
            bucket.last_successful_rate = Some(match bucket.last_successful_rate {
                Some(prev) => prev.max(bucket.current_rate),
                None => bucket.current_rate,
            });
        }
        let now = Instant::now();
        let mut esc = self.escalation.lock().expect("escalation mutex poisoned");
        if let Some(last_arm) = esc.last_arm {
            if now.saturating_duration_since(last_arm) > self.config.escalation_reset_after {
                esc.consecutive = 0;
                esc.last_arm = None;
            }
        }
    }

    async fn persist_aimd_state(&self, tuned_rate: f64, checkpoint_rate: f64) {
        let (last_arm_unix, consecutive) = self.escalation_snapshot();
        let ceiling = self.bucket.lock().await.discovered_ceiling.unwrap_or(0.0);
        self.persist_with_explicit_state(
            self.cooldown_until_unix.load(Ordering::Relaxed),
            consecutive,
            last_arm_unix,
            tuned_rate,
            checkpoint_rate,
            ceiling,
        );
    }

    fn escalation_snapshot(&self) -> (i64, u32) {
        let escalation = self.escalation.lock().expect("escalation mutex poisoned");
        let last_arm_unix = match escalation.last_arm {
            Some(t) => {
                chrono::Utc::now().timestamp()
                    - Instant::now().saturating_duration_since(t).as_secs() as i64
            }
            None => 0,
        };
        (last_arm_unix, escalation.consecutive)
    }

    fn persist_with_explicit_state(
        &self,
        until_unix: i64,
        consecutive: u32,
        last_arm_unix: i64,
        tuned_rate: f64,
        checkpoint_rate: f64,
        discovered_ceiling: f64,
    ) {
        let Some(path) = self.config.persistence_path.clone() else {
            return;
        };
        let state = PersistedState {
            cooldown_until_unix: until_unix,
            consecutive,
            last_arm_unix,
            tuned_rate,
            checkpoint_rate,
            discovered_ceiling,
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
        let elapsed = now
            .saturating_duration_since(self.last_refill)
            .as_secs_f64();
        if elapsed > 0.0 {
            self.tokens = (self.tokens + elapsed * rate).min(capacity);
            self.last_refill = now;
        }
    }
}

fn rate_to_interval_secs(rate: f64) -> f64 {
    if rate.is_finite() && rate > 0.0 {
        1.0 / rate
    } else {
        f64::INFINITY
    }
}

fn interval_to_rate(interval_secs: f64) -> f64 {
    if interval_secs.is_finite() && interval_secs > 0.0 {
        1.0 / interval_secs
    } else {
        0.0
    }
}

fn next_rate_for_shorter_interval(
    current_rate: f64,
    interval_step_secs: f64,
    max_rate: f64,
) -> f64 {
    if !current_rate.is_finite() || current_rate <= 0.0 {
        return max_rate.max(0.0);
    }
    let step = interval_step_secs.max(0.001);
    let current_interval = rate_to_interval_secs(current_rate);
    let max_interval = rate_to_interval_secs(max_rate);
    let next_interval = (current_interval - step).max(max_interval).max(0.001);
    interval_to_rate(next_interval).min(max_rate)
}

fn interval_precision_ceiling_candidate(
    failed_rate: f64,
    last_successful_rate: Option<f64>,
    interval_step_secs: f64,
) -> f64 {
    if let Some(safe) =
        last_successful_rate.filter(|safe| safe.is_finite() && *safe > 0.0 && *safe < failed_rate)
    {
        return safe;
    }
    let failed_interval = rate_to_interval_secs(failed_rate);
    if !failed_interval.is_finite() {
        return 0.0;
    }
    interval_to_rate(failed_interval + interval_step_secs.max(0.001))
}

fn restored_aimd_rate(state: &PersistedState) -> f64 {
    if state.tuned_rate > 0.0 {
        state.tuned_rate
    } else {
        state.checkpoint_rate
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
        // Faster refill so tests don't take seconds. AIMD is disabled
        // here so the bucket math stays deterministic; the AIMD-specific
        // tests below opt back in with their own configs.
        IapiLimiterConfig {
            capacity: 4.0,
            refill_per_sec: 40.0, // one token per 25 ms
            default_backoff_secs: 90,
            cloudflare_backoff_secs: 600,
            max_backoff_secs: 3600,
            escalation_reset_after: StdDuration::from_secs(60),
            persistence_path: None,
            aimd_enabled: false,
            aimd_min_rate: 1.0,
            aimd_max_rate: 100.0,
            aimd_increase_interval: StdDuration::from_secs(1),
            aimd_increment_per_step: 1.0,
            aimd_decrease_factor: 0.5,
            aimd_resolution: 0.01,
            aimd_tuned_after: StdDuration::from_secs(60),
            aimd_ceiling_headroom: 0.95,
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
        let secs = lim
            .record_rate_limited("<html>error code: 1015</html>")
            .await;
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
            tuned_rate: 0.0,
            checkpoint_rate: 0.0,
            discovered_ceiling: 0.0,
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
            tuned_rate: 0.0,
            checkpoint_rate: 0.0,
            discovered_ceiling: 0.0,
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
            tuned_rate: 0.0,
            checkpoint_rate: 0.0,
            discovered_ceiling: 0.0,
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

    #[tokio::test]
    async fn expired_rate_limit_persistence_does_not_freeze_aimd_ceiling() {
        let tmp = tempfile::NamedTempFile::new().expect("tempfile");
        let now_unix = chrono::Utc::now().timestamp();
        let state = PersistedState {
            cooldown_until_unix: now_unix - 60,
            consecutive: 0,
            last_arm_unix: 0,
            tuned_rate: 0.662_251_655_629_138_7,
            checkpoint_rate: 0.0,
            discovered_ceiling: 0.662_251_655_629_138_7,
        };
        std::fs::write(tmp.path(), serde_json::to_string(&state).unwrap()).unwrap();
        let cfg = IapiLimiterConfig {
            persistence_path: Some(tmp.path().to_path_buf()),
            ..aimd_config()
        };
        let lim = IapiLimiter::new(cfg);
        let bucket = lim.bucket.lock().await;
        assert!(
            !bucket.converged,
            "expired tuned state must not pause future AIMD ramp-up"
        );
        assert!(
            bucket.discovered_ceiling.is_none(),
            "stale discovered ceiling must not cap a fresh launch"
        );
    }

    /// Build a config tuned for AIMD behaviour tests: 10 ms increase
    /// interval and a large per-step jump so the ramp is observable in
    /// a single test without sleeping for seconds.
    fn aimd_config() -> IapiLimiterConfig {
        IapiLimiterConfig {
            capacity: 8.0,
            refill_per_sec: 1.0,
            default_backoff_secs: 90,
            cloudflare_backoff_secs: 600,
            max_backoff_secs: 3600,
            escalation_reset_after: StdDuration::from_secs(60),
            persistence_path: None,
            aimd_enabled: true,
            aimd_min_rate: 0.5,
            aimd_max_rate: 5.0,
            aimd_increase_interval: StdDuration::from_millis(10),
            aimd_increment_per_step: 1.0,
            aimd_decrease_factor: 0.5,
            // Tests use coarse 250ms pacing steps so ramp behaviour stays fast;
            // production default remains 10ms interval precision.
            aimd_resolution: 0.25,
            // Default for AIMD tests: long enough that ramp tests don't
            // accidentally hit convergence; convergence tests override
            // it explicitly via `convergence_config`.
            aimd_tuned_after: StdDuration::from_secs(60),
            aimd_ceiling_headroom: 0.95,
        }
    }

    #[tokio::test]
    async fn aimd_starts_at_refill_per_sec() {
        let lim = IapiLimiter::new(aimd_config());
        let rate = lim.current_rate_per_sec().await;
        assert!((rate - 1.0).abs() < 1e-6, "got {rate}");
    }

    #[tokio::test]
    async fn aimd_clamps_starting_rate_into_band() {
        // refill_per_sec sits below the configured floor — limiter should
        // clamp it up so the bucket isn't permanently stuck at a sub-min
        // rate before the first AIMD step.
        let cfg = IapiLimiterConfig {
            refill_per_sec: 0.1,
            aimd_min_rate: 0.5,
            ..aimd_config()
        };
        let lim = IapiLimiter::new(cfg);
        let rate = lim.current_rate_per_sec().await;
        assert!(rate >= 0.5, "got {rate}");
    }

    #[tokio::test]
    async fn aimd_ramps_up_after_clean_interval() {
        let lim = IapiLimiter::new(aimd_config());
        // Initial acquire — first interval hasn't elapsed yet so the
        // rate doesn't bump on this call.
        lim.acquire(1.0).await.expect("free");
        let r0 = lim.current_rate_per_sec().await;
        // Wait past the increase interval, then acquire again — the
        // acquire path is what triggers the AIMD bump.
        tokio::time::sleep(StdDuration::from_millis(25)).await;
        lim.acquire(1.0).await.expect("free");
        let r1 = lim.current_rate_per_sec().await;
        assert!(r1 > r0, "rate should have grown: r0={r0} r1={r1}");
    }

    #[tokio::test]
    async fn aimd_increase_caps_at_max_rate() {
        let lim = IapiLimiter::new(aimd_config());
        // Bang on the acquire path with sleeps between to allow many AIMD
        // steps. 30 iterations × +1 per step should saturate the 5.0 cap.
        for _ in 0..30 {
            tokio::time::sleep(StdDuration::from_millis(12)).await;
            lim.acquire(1.0).await.expect("free");
        }
        let rate = lim.current_rate_per_sec().await;
        assert!(
            (rate - 5.0).abs() < 1e-6,
            "should saturate at max 5.0, got {rate}"
        );
    }

    #[tokio::test]
    async fn aimd_halves_rate_on_429() {
        let lim = IapiLimiter::new(aimd_config());
        // Push rate up to ~3.0 first so the halving is observable.
        for _ in 0..2 {
            tokio::time::sleep(StdDuration::from_millis(12)).await;
            lim.acquire(1.0).await.expect("free");
        }
        let before = lim.current_rate_per_sec().await;
        assert!(before > 1.0, "expected rate > 1, got {before}");
        lim.record_rate_limited("Too Many Requests").await;
        let after = lim.current_rate_per_sec().await;
        assert!(
            (after - before * 0.5).abs() < 0.05,
            "rate should halve: before={before} after={after}"
        );
    }

    #[tokio::test]
    async fn aimd_decrease_floors_at_min_rate() {
        let lim = IapiLimiter::new(aimd_config());
        // Many 429s in a row — rate should clamp at aimd_min_rate (0.5).
        for _ in 0..10 {
            lim.record_rate_limited("Too Many Requests").await;
        }
        let rate = lim.current_rate_per_sec().await;
        assert!(
            (rate - 0.5).abs() < 1e-6,
            "should floor at min 0.5, got {rate}"
        );
    }

    #[tokio::test]
    async fn aimd_disabled_pins_rate_constant() {
        let mut cfg = aimd_config();
        cfg.aimd_enabled = false;
        cfg.refill_per_sec = 2.0;
        let lim = IapiLimiter::new(cfg);
        // Many acquires, but rate must stay at 2.0 since AIMD is off.
        for _ in 0..10 {
            tokio::time::sleep(StdDuration::from_millis(12)).await;
            lim.acquire(1.0).await.expect("free");
        }
        let rate = lim.current_rate_per_sec().await;
        assert!(
            (rate - 2.0).abs() < 1e-6,
            "rate should be pinned at 2.0, got {rate}"
        );
        // And 429 should also leave the rate unchanged when disabled.
        lim.record_rate_limited("Too Many Requests").await;
        let rate2 = lim.current_rate_per_sec().await;
        assert!(
            (rate2 - 2.0).abs() < 1e-6,
            "rate should stay at 2.0, got {rate2}"
        );
    }

    /// Config tuned for convergence tests: short `aimd_tuned_after` so
    /// the test doesn't have to sleep for 30 min.
    fn convergence_config() -> IapiLimiterConfig {
        IapiLimiterConfig {
            aimd_tuned_after: StdDuration::from_millis(40),
            // Make `aimd_increase_interval` larger than `aimd_tuned_after`
            // so a single quiet acquire reaches the tuned threshold
            // without the AIMD ramp accidentally resetting last_rate_change.
            aimd_increase_interval: StdDuration::from_millis(200),
            ..aimd_config()
        }
    }

    #[tokio::test]
    async fn aimd_converges_after_quiet_period_and_persists_rate() {
        let tmp = tempfile::NamedTempFile::new().expect("tempfile");
        let cfg = IapiLimiterConfig {
            persistence_path: Some(tmp.path().to_path_buf()),
            ..convergence_config()
        };
        let lim = IapiLimiter::new(cfg);
        // First acquire arms the bucket. Rate starts at refill_per_sec (1.0).
        lim.acquire(1.0).await.expect("free");
        // Wait past aimd_tuned_after, then trigger another acquire — the
        // convergence check fires inside acquire's bucket lock.
        tokio::time::sleep(StdDuration::from_millis(60)).await;
        lim.acquire(1.0).await.expect("free");
        // Persisted file should now carry the tuned rate.
        let raw = std::fs::read_to_string(tmp.path()).expect("persisted file");
        assert!(
            raw.contains("\"tuned_rate\""),
            "expected tuned_rate key in persisted state, got: {raw}"
        );
        let persisted: PersistedState = serde_json::from_str(&raw).expect("decode");
        assert!(persisted.tuned_rate > 0.0, "tuned_rate must be > 0");
    }

    #[tokio::test]
    async fn aimd_checkpoint_persists_on_clean_increase_without_tuned_rate() {
        let tmp = tempfile::NamedTempFile::new().expect("tempfile");
        let cfg = IapiLimiterConfig {
            persistence_path: Some(tmp.path().to_path_buf()),
            ..aimd_config()
        };
        let lim = IapiLimiter::new(cfg);
        lim.acquire(1.0).await.expect("free");
        tokio::time::sleep(StdDuration::from_millis(25)).await;
        lim.acquire(1.0).await.expect("free");
        let live = lim.current_rate_per_sec().await;
        let raw = std::fs::read_to_string(tmp.path()).expect("persisted checkpoint");
        let persisted: PersistedState = serde_json::from_str(&raw).expect("decode");
        assert_eq!(
            persisted.tuned_rate, 0.0,
            "clean increase is only a checkpoint"
        );
        assert!(
            (persisted.checkpoint_rate - live).abs() < 1e-6,
            "checkpoint {} should match live {}",
            persisted.checkpoint_rate,
            live
        );
    }

    #[tokio::test]
    async fn aimd_restores_checkpoint_rate_without_converging() {
        let tmp = tempfile::NamedTempFile::new().expect("tempfile");
        let persisted = PersistedState {
            cooldown_until_unix: 0,
            consecutive: 0,
            last_arm_unix: 0,
            tuned_rate: 0.0,
            checkpoint_rate: 2.25,
            discovered_ceiling: 0.0,
        };
        std::fs::write(tmp.path(), serde_json::to_string(&persisted).unwrap()).unwrap();
        let cfg = IapiLimiterConfig {
            persistence_path: Some(tmp.path().to_path_buf()),
            refill_per_sec: 1.0,
            ..convergence_config()
        };
        let lim = IapiLimiter::new(cfg);
        let rate = lim.current_rate_per_sec().await;
        assert!(
            (rate - 2.25).abs() < 1e-6,
            "expected 2.25 restored, got {rate}"
        );
        let bucket = lim.bucket.lock().await;
        assert!(
            !bucket.converged,
            "checkpoint restore must keep AIMD active"
        );
    }

    #[tokio::test]
    async fn aimd_restores_tuned_rate_on_construction() {
        let tmp = tempfile::NamedTempFile::new().expect("tempfile");
        // Pre-seed a persisted tuned rate of 3.5.
        let persisted = PersistedState {
            cooldown_until_unix: 0,
            consecutive: 0,
            last_arm_unix: 0,
            tuned_rate: 3.5,
            checkpoint_rate: 0.0,
            discovered_ceiling: 0.0,
        };
        std::fs::write(tmp.path(), serde_json::to_string(&persisted).unwrap()).unwrap();
        let cfg = IapiLimiterConfig {
            persistence_path: Some(tmp.path().to_path_buf()),
            // Cold default is 1.0, but persisted 3.5 should win.
            refill_per_sec: 1.0,
            ..convergence_config()
        };
        let lim = IapiLimiter::new(cfg);
        let rate = lim.current_rate_per_sec().await;
        assert!(
            (rate - 3.5).abs() < 1e-6,
            "expected 3.5 restored, got {rate}"
        );
    }

    #[tokio::test]
    async fn aimd_429_clears_converged_flag_and_decreases_rate() {
        let tmp = tempfile::NamedTempFile::new().expect("tempfile");
        let cfg = IapiLimiterConfig {
            persistence_path: Some(tmp.path().to_path_buf()),
            ..convergence_config()
        };
        let lim = IapiLimiter::new(cfg);
        // Force convergence.
        lim.acquire(1.0).await.expect("free");
        tokio::time::sleep(StdDuration::from_millis(60)).await;
        lim.acquire(1.0).await.expect("free");
        let pre = lim.current_rate_per_sec().await;
        // 429 should halve the rate and clear `converged` so subsequent
        // clean traffic ramps again.
        lim.record_rate_limited("Too Many Requests").await;
        let post = lim.current_rate_per_sec().await;
        assert!(
            post < pre,
            "rate should decrease after 429: pre={pre} post={post}"
        );
        // Persisted state should reflect the post-429 rate as a checkpoint,
        // not as a converged/tuned value.
        let raw = std::fs::read_to_string(tmp.path()).expect("persisted file");
        let persisted: PersistedState = serde_json::from_str(&raw).expect("decode");
        assert_eq!(
            persisted.tuned_rate, 0.0,
            "429 must clear tuned/converged state"
        );
        assert!(
            (persisted.checkpoint_rate - post).abs() < 1e-6,
            "post-429 checkpoint rate {} should match live {}",
            persisted.checkpoint_rate,
            post
        );
    }

    #[test]
    fn legacy_persisted_state_without_tuned_rate_decodes() {
        // Pre-AIMD persistence files don't have tuned_rate or
        // discovered_ceiling — serde defaults keep them decoding cleanly.
        let legacy =
            r#"{"cooldown_until_unix":1700000000,"consecutive":1,"last_arm_unix":1699999000}"#;
        let s: PersistedState = serde_json::from_str(legacy).expect("legacy decode");
        assert_eq!(s.cooldown_until_unix, 1_700_000_000);
        assert_eq!(s.tuned_rate, 0.0);
        assert_eq!(s.checkpoint_rate, 0.0);
        assert_eq!(s.discovered_ceiling, 0.0);
    }

    #[tokio::test]
    async fn ssthresh_sets_ceiling_one_interval_step_slower_on_first_429() {
        let lim = IapiLimiter::new(aimd_config());
        // Ramp up to ~4.0 req/s so the 429 lands on a non-default rate.
        for _ in 0..3 {
            tokio::time::sleep(StdDuration::from_millis(12)).await;
            lim.acquire(1.0).await.expect("free");
        }
        let pre_rate = lim.current_rate_per_sec().await;
        assert!(pre_rate >= 2.5, "expected ramp >= 2.5, got {pre_rate}");
        lim.record_rate_limited("error code: 1015").await;
        let bucket = lim.bucket.lock().await;
        let ceiling = bucket.discovered_ceiling.expect("ceiling set after 429");
        let expected = interval_to_rate(rate_to_interval_secs(pre_rate) + 0.25).max(0.5);
        assert!(
            (ceiling - expected).abs() < 0.01,
            "expected ceiling ≈ {expected}, got {ceiling}"
        );
    }

    #[tokio::test]
    async fn ssthresh_uses_last_successful_rate_for_precise_ceiling() {
        let lim = IapiLimiter::new(aimd_config());
        {
            let mut bucket = lim.bucket.lock().await;
            bucket.current_rate = 1.12;
        }
        lim.record_success().await;
        {
            let mut bucket = lim.bucket.lock().await;
            bucket.current_rate = 1.13;
        }
        lim.record_rate_limited("error code: 1015").await;
        let ceiling = lim.bucket.lock().await.discovered_ceiling.unwrap();
        assert!(
            (ceiling - 1.12).abs() < 1e-6,
            "expected precise 1.12 req/s ceiling, got {ceiling}"
        );
    }

    #[tokio::test]
    async fn ssthresh_without_last_success_uses_hundredth_second_pacing_precision() {
        let cfg = IapiLimiterConfig {
            aimd_resolution: 0.01,
            ..aimd_config()
        };
        let lim = IapiLimiter::new(cfg);
        {
            let mut bucket = lim.bucket.lock().await;
            bucket.current_rate = 1.13;
            bucket.last_successful_rate = None;
        }
        lim.record_rate_limited("error code: 1015").await;
        let ceiling = lim.bucket.lock().await.discovered_ceiling.unwrap();
        let expected = interval_to_rate(rate_to_interval_secs(1.13) + 0.01);
        assert!(
            (ceiling - expected).abs() < 1e-9,
            "expected 0.01s slower pacing ceiling {expected}, got {ceiling}"
        );
    }

    #[tokio::test]
    async fn ssthresh_ramp_stops_at_discovered_ceiling() {
        let lim = IapiLimiter::new(aimd_config());
        // Ramp up + 429 to set a ceiling around 2.85 req/s.
        for _ in 0..2 {
            tokio::time::sleep(StdDuration::from_millis(12)).await;
            lim.acquire(1.0).await.expect("free");
        }
        let pre = lim.current_rate_per_sec().await;
        lim.record_rate_limited("error code: 1015").await;
        let ceiling = lim.bucket.lock().await.discovered_ceiling.unwrap();
        let expected_ceiling = interval_to_rate(rate_to_interval_secs(pre) + 0.25).max(0.5);
        assert!((ceiling - expected_ceiling).abs() < 0.01);
        // Clear the 1015 cooldown so the acquire loop can run again —
        // we're testing the ramp cap behavior, not cooldown gating.
        lim.cooldown_until_unix.store(0, Ordering::Relaxed);
        // Now ramp back up. The rate must NOT exceed the discovered
        // ceiling even after many clean intervals.
        for _ in 0..30 {
            tokio::time::sleep(StdDuration::from_millis(12)).await;
            lim.acquire(1.0).await.expect("free");
        }
        let post = lim.current_rate_per_sec().await;
        assert!(
            post <= ceiling + 0.001,
            "ramp should cap at ceiling {ceiling}, got {post}"
        );
    }

    #[tokio::test]
    async fn ssthresh_subsequent_429_only_lowers_ceiling() {
        let lim = IapiLimiter::new(aimd_config());
        // First 429 at ~3 req/s → ceiling ~2.85
        for _ in 0..3 {
            tokio::time::sleep(StdDuration::from_millis(12)).await;
            lim.acquire(1.0).await.expect("free");
        }
        lim.record_rate_limited("error code: 1015").await;
        let first_ceiling = lim.bucket.lock().await.discovered_ceiling.unwrap();
        // Force the rate higher than first_ceiling by clearing the ceiling
        // and ramping to a higher value, then re-applying — to simulate
        // Cloudflare's behavior staying the same (next 429 fires at a
        // higher rate that's still above prior ceiling). The merge logic
        // should keep the LOWER ceiling since that's the worst-known limit.
        {
            let mut bucket = lim.bucket.lock().await;
            bucket.current_rate = (first_ceiling + 1.0).min(5.0);
            bucket.discovered_ceiling = Some(first_ceiling);
        }
        // Now a 429 at higher rate would compute a higher candidate ceiling,
        // but merge with existing keeps the lower one.
        lim.record_rate_limited("error code: 1015").await;
        let merged = lim.bucket.lock().await.discovered_ceiling.unwrap();
        assert!(
            merged <= first_ceiling + 0.001,
            "ceiling should not move up: first={first_ceiling} merged={merged}"
        );
    }

    #[tokio::test]
    async fn ssthresh_persists_ceiling_across_restart() {
        let tmp = tempfile::NamedTempFile::new().expect("tempfile");
        let cfg = IapiLimiterConfig {
            persistence_path: Some(tmp.path().to_path_buf()),
            ..aimd_config()
        };
        let lim = IapiLimiter::new(cfg.clone());
        // Ramp + 429 → ceiling persists
        for _ in 0..2 {
            tokio::time::sleep(StdDuration::from_millis(12)).await;
            lim.acquire(1.0).await.expect("free");
        }
        lim.record_rate_limited("error code: 1015").await;
        let original_ceiling = lim.bucket.lock().await.discovered_ceiling.unwrap();
        drop(lim);
        // Rebuild from the persisted file — fresh process simulation.
        let lim2 = IapiLimiter::new(cfg);
        let restored = lim2.bucket.lock().await.discovered_ceiling.unwrap_or(0.0);
        assert!(
            (restored - original_ceiling).abs() < 0.01,
            "ceiling should restore: original={original_ceiling} restored={restored}"
        );
    }
}
