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
        aimd_low_rate_increment_per_step: 0.01,
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
async fn concurrent_arms_coalesce_into_one_escalation() {
    // A burst of 429s from one overshoot lands while the first backoff is
    // still armed, so they must NOT compound — keep the single base window
    // instead of doubling per call (the bug that produced 1-hour bans).
    let lim = IapiLimiter::new(fast_config());
    let first = lim.record_rate_limited("Too Many Requests").await;
    let second = lim.record_rate_limited("Too Many Requests").await;
    let third = lim.record_rate_limited("Too Many Requests").await;
    assert!((89..=90).contains(&first), "got {first}");
    assert_eq!(
        second, first,
        "concurrent arm must not escalate: {first}/{second}"
    );
    assert_eq!(
        third, first,
        "concurrent arm must not escalate: {first}/{third}"
    );
}

#[tokio::test]
async fn separated_overshoots_escalate_once_each() {
    // A distinct overshoot after the armed window clears is a real repeat —
    // it escalates by one (not coalesced).
    let mut cfg = fast_config();
    cfg.default_backoff_secs = 1; // tiny so the armed window clears fast
    let lim = IapiLimiter::new(cfg);
    let first = lim.record_rate_limited("Too Many Requests").await;
    assert_eq!(first, 1, "got {first}");
    tokio::time::sleep(StdDuration::from_millis(1_100)).await;
    let second = lim.record_rate_limited("Too Many Requests").await;
    assert_eq!(
        second, 2,
        "a fresh overshoot after the window escalates once"
    );
}

#[tokio::test]
async fn escalation_capped_at_max() {
    let mut cfg = fast_config();
    cfg.max_backoff_secs = 500;
    let lim = IapiLimiter::new(cfg);
    // 30 arms from one overshoot burst coalesce to a single base window,
    // nowhere near the cap — the cascade that produced hour-long bans.
    for _ in 0..30 {
        lim.record_rate_limited("Too Many Requests").await;
    }
    let remaining = lim.remaining_backoff_secs().unwrap();
    assert!(
        remaining <= 91,
        "burst must coalesce to one base window, got {remaining}"
    );
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
async fn restore_clamps_checkpoint_to_discovered_ceiling_during_cooldown() {
    let tmp = tempfile::NamedTempFile::new().expect("tempfile");
    let now_unix = chrono::Utc::now().timestamp();
    let state = PersistedState {
        cooldown_until_unix: now_unix + 300,
        consecutive: 1,
        last_arm_unix: now_unix - 5,
        tuned_rate: 0.0,
        checkpoint_rate: 125.0,
        discovered_ceiling: 71.4286,
    };
    std::fs::write(tmp.path(), serde_json::to_string(&state).unwrap()).unwrap();
    let cfg = IapiLimiterConfig {
        persistence_path: Some(tmp.path().to_path_buf()),
        refill_per_sec: 0.67,
        aimd_max_rate: 250.0,
        ..aimd_config()
    };
    let lim = IapiLimiter::new(cfg);
    let rate = lim.current_rate_per_sec().await;
    assert!(
        rate <= 71.4286 + 0.001,
        "restored rate must not exceed discovered ceiling, got {rate}"
    );
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

#[tokio::test]
async fn stale_high_checkpoint_does_not_restart_at_cloudflare_ban_rate() {
    let tmp = tempfile::NamedTempFile::new().expect("tempfile");
    let now_unix = chrono::Utc::now().timestamp();
    let state = PersistedState {
        cooldown_until_unix: now_unix - 60,
        consecutive: 0,
        last_arm_unix: 0,
        tuned_rate: 0.0,
        checkpoint_rate: 250.0,
        discovered_ceiling: 0.0,
    };
    std::fs::write(tmp.path(), serde_json::to_string(&state).unwrap()).unwrap();
    let cfg = IapiLimiterConfig {
        persistence_path: Some(tmp.path().to_path_buf()),
        refill_per_sec: 0.67,
        aimd_max_rate: 250.0,
        ..aimd_config()
    };
    let lim = IapiLimiter::new(cfg);
    let rate = lim.current_rate_per_sec().await;
    assert!(
        rate <= 1.0,
        "stale high checkpoint must restart conservatively, got {rate}"
    );
}

#[tokio::test]
async fn stale_checkpoint_above_lowered_ceiling_reprobes_from_floor() {
    // Production regression: a 250 req/s checkpoint was persisted from a
    // session that ramped to the old guard without real concurrency, and the
    // user restarts inside the AIMD learning TTL (recent_aimd_learning=true)
    // so the restore path would otherwise resume at 250 → clamp to the new
    // 40 ceiling → blast 40 req/s on launch. The persisted rate is above the
    // ceiling, so it must be discarded and re-probed from refill_per_sec.
    let tmp = tempfile::NamedTempFile::new().expect("tempfile");
    let now_unix = chrono::Utc::now().timestamp();
    let state = PersistedState {
        cooldown_until_unix: now_unix - 60,
        consecutive: 0,
        last_arm_unix: now_unix - 5, // recent → recent_aimd_learning = true
        tuned_rate: 0.0,
        checkpoint_rate: 250.0,
        discovered_ceiling: 0.0,
    };
    std::fs::write(tmp.path(), serde_json::to_string(&state).unwrap()).unwrap();
    let cfg = IapiLimiterConfig {
        persistence_path: Some(tmp.path().to_path_buf()),
        refill_per_sec: 5.0,
        aimd_max_rate: 40.0,
        ..aimd_config()
    };
    let lim = IapiLimiter::new(cfg);
    let rate = lim.current_rate_per_sec().await;
    assert!(
        (rate - 5.0).abs() < 1e-6,
        "stale 250 checkpoint above the 40 ceiling must re-probe from refill_per_sec (5.0), not resume at the ceiling, got {rate}"
    );
}

#[tokio::test]
async fn expired_cooldown_keeps_recent_discovered_ceiling_learning() {
    let tmp = tempfile::NamedTempFile::new().expect("tempfile");
    let now_unix = chrono::Utc::now().timestamp();
    let state = PersistedState {
        cooldown_until_unix: now_unix - 60,
        consecutive: 1,
        // Older than the escalation reset window, but still inside the
        // AIMD learning TTL. This is the normal "restart shortly after a
        // 600s Cloudflare 1015 backoff elapsed" case.
        last_arm_unix: now_unix - 700,
        tuned_rate: 0.0,
        checkpoint_rate: 0.7534,
        discovered_ceiling: 0.7634,
    };
    std::fs::write(tmp.path(), serde_json::to_string(&state).unwrap()).unwrap();
    let cfg = IapiLimiterConfig {
        persistence_path: Some(tmp.path().to_path_buf()),
        refill_per_sec: 0.5,
        aimd_tuned_after: StdDuration::from_secs(30 * 60),
        ..aimd_config()
    };
    let lim = IapiLimiter::new(cfg);
    let rate = lim.current_rate_per_sec().await;
    let ceiling = lim.bucket.lock().await.discovered_ceiling;
    assert!(
        (rate - 0.7534).abs() < 0.0001,
        "recent checkpoint should survive cooldown expiry, got {rate}"
    );
    assert!(
        ceiling
            .map(|c| (c - 0.7634).abs() < 0.0001)
            .unwrap_or(false),
        "recent discovered ceiling should still cap the next ramp, got {ceiling:?}"
    );
}

#[tokio::test]
async fn recent_floor_checkpoint_restores_near_discovered_ceiling() {
    let tmp = tempfile::NamedTempFile::new().expect("tempfile");
    let now_unix = chrono::Utc::now().timestamp();
    let state = PersistedState {
        cooldown_until_unix: now_unix - 60,
        consecutive: 1,
        // Recent 1015 learning from older limiter builds may have persisted
        // the post-limit checkpoint at the floor even though the precise
        // discovered ceiling was known. Startup should use that ceiling to
        // avoid re-crawling from 0.5 req/s every run.
        last_arm_unix: now_unix - 700,
        tuned_rate: 0.0,
        checkpoint_rate: 0.5,
        discovered_ceiling: 0.7634,
    };
    std::fs::write(tmp.path(), serde_json::to_string(&state).unwrap()).unwrap();
    let cfg = IapiLimiterConfig {
        persistence_path: Some(tmp.path().to_path_buf()),
        refill_per_sec: 0.5,
        aimd_tuned_after: StdDuration::from_secs(30 * 60),
        ..aimd_config()
    };
    let lim = IapiLimiter::new(cfg);
    let rate = lim.current_rate_per_sec().await;
    assert!(
        (rate - 0.7534).abs() < 0.0001,
        "recent floor checkpoint should restart near ceiling, got {rate}"
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
        aimd_low_rate_increment_per_step: 0.01,
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
async fn aimd_recovers_near_discovered_ceiling_after_429() {
    let lim = IapiLimiter::new(aimd_config());
    // Push rate up first so precision ceiling discovery has useful headroom.
    for _ in 0..2 {
        tokio::time::sleep(StdDuration::from_millis(12)).await;
        lim.acquire(1.0).await.expect("free");
    }
    let before = lim.current_rate_per_sec().await;
    assert!(before > 1.0, "expected rate > 1, got {before}");
    lim.record_rate_limited("Too Many Requests").await;
    let after = lim.current_rate_per_sec().await;
    let ceiling = interval_precision_ceiling_candidate(before, None, 0.25);
    assert!(
        after >= ceiling * 0.90 && after <= ceiling,
        "rate should restart just under discovered ceiling: before={before} ceiling={ceiling} after={after}"
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
async fn aimd_restores_recent_checkpoint_rate_without_converging() {
    let tmp = tempfile::NamedTempFile::new().expect("tempfile");
    let now_unix = chrono::Utc::now().timestamp();
    let persisted = PersistedState {
        cooldown_until_unix: 0,
        consecutive: 1,
        last_arm_unix: now_unix - 5,
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
    let legacy = r#"{"cooldown_until_unix":1700000000,"consecutive":1,"last_arm_unix":1699999000}"#;
    let s: PersistedState = serde_json::from_str(legacy).expect("legacy decode");
    assert_eq!(s.cooldown_until_unix, 1_700_000_000);
    assert_eq!(s.tuned_rate, 0.0);
    assert_eq!(s.checkpoint_rate, 0.0);
    assert_eq!(s.discovered_ceiling, 0.0);
}

#[test]
fn aimd_interval_ramp_switches_to_additive_above_resolution_floor() {
    let max_rate = 250.0;
    let at_resolution = interval_to_rate(0.01);
    let first = next_rate_for_shorter_interval(at_resolution, 0.01, 5.0, 0.01, max_rate);
    assert!(
        (first - 105.0).abs() < 1e-9,
        "should add static req/s above 0.01s pacing, got {first}"
    );
    let second = next_rate_for_shorter_interval(first, 0.01, 5.0, 0.01, max_rate);
    assert!(
        (second - 110.0).abs() < 1e-9,
        "should continue static probing, got {second}"
    );
}

#[test]
fn aimd_interval_ramp_does_not_jump_straight_to_high_max_rate() {
    let next = next_rate_for_shorter_interval(100.0, 0.01, 5.0, 0.01, 1_000.0);
    assert!(
        (next - 105.0).abs() < 1e-9,
        "0.01s pacing should add one rate step, not jump to max; got {next}"
    );
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
