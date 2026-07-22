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
    /// on a long clean run. This is a safety guard, not a guessed Kraken cap:
    /// AIMD should keep probing until Cloudflare pushes back or this high
    /// operational guard is reached.
    pub aimd_max_rate: f64,
    /// How often (during clean traffic) to tighten request pacing by one
    /// `aimd_resolution` interval step while pacing is coarser than the
    /// resolution. Once pacing reaches that resolution floor (for example
    /// 0.01s = 100 req/s), AIMD switches to static req/s increments so it can
    /// keep probing higher rates without collapsing interval math to zero.
    pub aimd_increase_interval: Duration,
    /// Additive req/s increment used after the interval-step ramp reaches the
    /// configured pacing resolution. This keeps probing above 0.01s pacing
    /// controlled instead of jumping straight to `aimd_max_rate`.
    pub aimd_increment_per_step: f64,
    /// Low-rate additive req/s increment used below 1 req/s. At iapi's observed
    /// equity-history ceiling, interval-step probing makes ~0.014 req/s jumps
    /// (0.840 → 0.855), which is coarser than the useful decision boundary.
    /// Probe that final band directly in hundredth-req/s increments instead.
    pub aimd_low_rate_increment_per_step: f64,
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
            // Small bucket: near the ~6 req/s iapi ceiling a large burst just
            // fires several requests over the limit at once, each a 429. Keeping
            // capacity low bounds how many concurrent calls can overshoot before
            // the rate decrease lands (the escalation coalescer handles the rest).
            capacity: 4.0,
            // Cold-start / re-probe floor. Live probing showed the real iapi
            // ceiling is ~6 req/s (sustained 7+ Cloudflare-1015'd), so start just
            // under it rather than crawling up from sub-1 req/s; AIMD still
            // halves on any 429.
            refill_per_sec: 5.0,
            default_backoff_secs: 90,
            cloudflare_backoff_secs: 600,
            max_backoff_secs: 3600,
            escalation_reset_after: Duration::from_secs(600),
            persistence_path: None,
            aimd_enabled: true,
            aimd_min_rate: 0.5,
            // Operational ceiling. An earlier 120 guard was a mistake: driving
            // the catalog at sustained volume, Cloudflare 1015'd at ~11 req/s and
            // the discovered ceiling settled near 6–7 req/s — the "clean ramp to
            // 40" was a mirage at trickle volume. Cap the guard just above the
            // real edge; the persisted `discovered_ceiling` pins the converged
            // rate below it. iapi is fundamentally a ~6 req/s lane, so a full
            // ~100k-fetch sweep is hours, not minutes — request reduction (TF
            // derivation), not a higher cap, is the only real speedup.
            aimd_max_rate: 10.0,
            // Gentle approach: we start (~5) right under the ceiling, so a slow
            // step probes the last req/s without lunging past it into a 1015.
            aimd_increase_interval: Duration::from_secs(10),
            aimd_increment_per_step: 5.0,
            aimd_low_rate_increment_per_step: 0.01,
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
    /// Unix deadline of the backoff most recently armed. Concurrent 429s from a
    /// single overshoot (the bucket lets a burst fire before the rate decrease
    /// lands) all observe this future deadline and coalesce into one escalation
    /// instead of compounding — without it, ~8 simultaneous 429s drove the
    /// window to `base × 2^7` (a capped 1-hour ban) on one overshoot.
    armed_until_unix: i64,
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
                            // Coalesce any burst that fires the instant a
                            // restored in-flight ban expires.
                            armed_until_unix: state.cooldown_until_unix,
                        },
                        restored_aimd_rate(state),
                        state.tuned_rate > 0.0,
                        state.discovered_ceiling,
                    )
                }
                Some(state) => {
                    let age_secs = (state.last_arm_unix > 0 && state.last_arm_unix <= now_unix)
                        .then_some((now_unix - state.last_arm_unix) as u64);
                    // Keep empirical AIMD learning longer than the escalation
                    // counter. The 1015 cooldown itself is 600s; if the user
                    // restarts shortly after that window expires, dropping the
                    // learned ~0.75 req/s checkpoint forces another long crawl
                    // from 0.5 req/s and another ceiling probe. `aimd_tuned_after`
                    // is a bounded learning TTL here: long enough to survive a
                    // cooldown/restart cycle, short enough that stale IP/session
                    // observations do not freeze future launches forever.
                    let recent_aimd_learning = age_secs
                        .map(|age| {
                            age <= config
                                .escalation_reset_after
                                .max(config.aimd_tuned_after)
                                .as_secs()
                        })
                        .unwrap_or(false);
                    let restored_rate = restored_aimd_rate(state);
                    let restored_rate = if recent_aimd_learning || state.tuned_rate > 0.0 {
                        if recent_aimd_learning
                            && state.tuned_rate <= 0.0
                            && state.discovered_ceiling > config.aimd_min_rate
                        {
                            // Older persisted states may carry a precise
                            // discovered ceiling but a floor-level checkpoint
                            // from the post-429 decrease. Resume near the
                            // learned safe edge instead of spending another
                            // long session crawling from the floor.
                            restored_rate.max(recovery_rate_after_limit(
                                restored_rate,
                                state.discovered_ceiling,
                                config.aimd_decrease_factor,
                                config.aimd_min_rate,
                                config.aimd_low_rate_increment_per_step,
                            ))
                        } else {
                            restored_rate
                        }
                    } else {
                        restored_rate.min(config.refill_per_sec.max(config.aimd_min_rate))
                    };
                    (
                        0,
                        EscalationState::default(),
                        restored_rate,
                        false,
                        if recent_aimd_learning {
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
        // A persisted rate above the current safety ceiling was learned under a
        // higher ceiling or at unrealistic concurrency (e.g. the old 250 req/s
        // checkpoint that the 2-permit broker could never actually drive). Drop
        // it and re-probe from `refill_per_sec` so AIMD rediscovers the real
        // Cloudflare ceiling under the current concurrency model instead of
        // resuming at a rate that was never validated.
        let restored_rate = if restored_rate > config.aimd_max_rate {
            tracing::info!(
                "iapi AIMD: discarding stale persisted rate {:.2} req/s above ceiling {:.2} req/s — re-probing from {:.2} req/s",
                restored_rate,
                config.aimd_max_rate,
                config.refill_per_sec
            );
            0.0
        } else {
            restored_rate
        };
        let raw_starting_rate = if restored_rate > 0.0 {
            restored_rate
        } else {
            config.refill_per_sec
        };
        let mut starting_rate = raw_starting_rate
            .max(config.aimd_min_rate)
            .min(config.aimd_max_rate);
        if let Some(ceiling) = starting_ceiling {
            starting_rate = starting_rate.min(ceiling);
        }
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
                // back into the known ban zone. The discovered ceiling is the
                // *last rate that tripped a limit*, so converging exactly at it
                // re-trips on the next clean ramp (the observed 5.4→5.1→4.9
                // ceiling decay with 600–1200 s 1015 backoffs each cycle). Hold a
                // `aimd_ceiling_headroom` margin below it so we settle just under
                // the wall instead of repeatedly probing into it.
                let effective_max = bucket
                    .discovered_ceiling
                    .map(|c| (c * self.config.aimd_ceiling_headroom).min(self.config.aimd_max_rate))
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
                        self.config.aimd_increment_per_step,
                        self.config.aimd_low_rate_increment_per_step,
                        effective_max,
                    );
                    if (new_rate - bucket.current_rate).abs() >= 0.000_001 {
                        tracing::info!(
                            "iapi AIMD: rate ↑ {:.4} → {:.4} req/s after clean run",
                            bucket.current_rate,
                            new_rate
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
        let now_unix = chrono::Utc::now().timestamp();
        let (chosen_secs, consecutive) = {
            let mut esc = self.escalation.lock().expect("escalation mutex poisoned");
            // Coalesce concurrent 429s from a single overshoot. The token bucket
            // lets a burst fire before the multiplicative-decrease lands, so up
            // to `capacity` requests can 429 within milliseconds. Escalating once
            // per 429 turned one overshoot into an hour-long ban (600 × 2^7,
            // capped). If a backoff armed under this same lock by a sibling of the
            // same overshoot is still active, this 429 is that same event: keep
            // `consecutive`, only extend the window. Real repeat overshoots can
            // only happen after the backoff expires (acquire() short-circuits
            // while it is active), at which point `armed_until_unix` is in the
            // past and escalation resumes.
            let same_overshoot = now_unix < esc.armed_until_unix;
            let consecutive = if same_overshoot {
                esc.consecutive.max(1)
            } else if esc
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
            // Record the armed deadline so concurrent siblings coalesce above.
            esc.armed_until_unix = esc.armed_until_unix.max(now_unix.saturating_add(capped));
            (capped, consecutive)
        };
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
                let new_rate = recovery_rate_after_limit(
                    bucket.current_rate,
                    merged,
                    self.config.aimd_decrease_factor,
                    self.config.aimd_min_rate,
                    self.config.aimd_low_rate_increment_per_step,
                );
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
            armed_until_unix: 0,
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
    additive_rate_step: f64,
    low_rate_step: f64,
    max_rate: f64,
) -> f64 {
    if !current_rate.is_finite() || current_rate <= 0.0 {
        return max_rate.max(0.0);
    }
    if current_rate < 1.0 {
        let rate_step = low_rate_step.max(0.001);
        return (current_rate + rate_step).min(max_rate);
    }
    let step = interval_step_secs.max(0.001);
    let current_interval = rate_to_interval_secs(current_rate);
    if !current_interval.is_finite() {
        return max_rate.max(0.0);
    }
    // Interval-step discovery is ideal while pacing is coarser than the
    // configured precision. At or below that precision, subtracting another
    // interval step would hit zero/negative spacing and jump straight to the
    // max. Switch to bounded additive req/s probing instead.
    if current_interval <= step {
        let rate_step = additive_rate_step.max(0.001);
        return (current_rate + rate_step).min(max_rate);
    }
    let max_interval = rate_to_interval_secs(max_rate);
    let next_interval = (current_interval - step).max(max_interval).max(step);
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

fn recovery_rate_after_limit(
    failed_rate: f64,
    discovered_ceiling: f64,
    decrease_factor: f64,
    min_rate: f64,
    low_rate_step: f64,
) -> f64 {
    let multiplicative = (failed_rate * decrease_factor).max(min_rate);
    if discovered_ceiling.is_finite() && discovered_ceiling > min_rate {
        // After precision-based ceiling discovery, restart one final-band probe
        // below the known safe edge instead of throwing away the whole learned
        // ramp. For the observed ~0.84 req/s iapi ceiling this resumes around
        // 0.83 req/s, then probes 0.84/0.85 in 0.01 req/s steps.
        let near_ceiling = (discovered_ceiling - low_rate_step.max(0.001)).max(min_rate);
        return multiplicative.max(near_ceiling).min(discovered_ceiling);
    }
    multiplicative
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
mod tests;
