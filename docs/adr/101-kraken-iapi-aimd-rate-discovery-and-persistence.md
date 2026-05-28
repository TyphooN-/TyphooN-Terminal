# ADR-101: Kraken iapi AIMD Rate Discovery and Persistence

**Status:** Accepted | **Date:** 2026-05-27

## Context

Kraken Securities / equity-style market-data access uses Kraken's internal
`iapi.kraken.com` endpoints rather than the documented public Spot OHLC REST
budget covered by ADR-095. These endpoints sit behind Cloudflare and can apply
IP-level throttling. When the app overdrives them, failures can appear as HTTP
429, Cloudflare 1015 HTML bodies, or backend instability that affects multiple
iapi call sites at once.

A fixed request rate is the wrong fit for this surface:

- the safe ceiling can vary by session, IP, Cloudflare state, endpoint mix, and
  recent traffic history;
- starting every app launch from a conservative cold rate wastes sync time after
  the previous session already discovered a safe operating point;
- restoring a rate as if it were final is also unsafe because Cloudflare can
  tighten the ceiling later;
- broad Kraken Securities coverage creates a large backlog, so losing in-memory
  limiter learning on every restart materially slows progress toward full sync.

TyphooN therefore needs iapi-specific empirical rate discovery that survives
restarts without pretending every restored rate is a converged ceiling.

## Decision

Use a process-wide AIMD limiter for all Kraken iapi calls.

AIMD means Additive Increase / Multiplicative Decrease:

- on clean traffic, the limiter tightens pacing by one configured interval step
  (`aimd_resolution`, currently 0.01 seconds) every `aimd_increase_interval`;
- on HTTP 429 / Cloudflare 1015, the limiter multiplicatively decreases the live
  rate (`aimd_decrease_factor`, currently 0.5), drains tokens, and arms a
  cooldown;
- the live rate is clamped by configured floor/ceiling bounds so it cannot fall
  to zero or ramp without limit. The default ceiling is intentionally a safety
  cap, not a statement of the provider's maximum; clean sessions may need to
  probe beyond 5 req/s, and `TYPHOON_KRAKEN_IAPI_AIMD_MAX_RATE` can raise or
  lower the cap for controlled experiments.

The limiter persists three separate concepts:

1. `checkpoint_rate`
   - written on each clean AIMD increase and after 429-triggered decreases;
   - restored on next launch as the current rate;
   - does **not** mark the limiter converged;
   - keeps AIMD active so the app continues probing from the latest known point.

2. `tuned_rate`
   - written only after the live rate has stayed unchanged for
     `aimd_tuned_after`;
   - restored as a converged/tuned rate;
   - still remains subject to future 429 decreases because provider ceilings can
     change.

3. `discovered_ceiling`
   - written when a 429 / 1015 establishes the highest known unsafe zone;
   - restored on next launch;
   - caps future clean ramps below the known ban zone instead of rediscovering
     the same failure repeatedly.

Cooldown state and escalation counters remain persisted as before, so a restart
inside an active Cloudflare backoff does not immediately hammer iapi again.

## Implementation Notes

- Source: `engine/src/broker/kraken/iapi_limiter.rs`.
- The persistence file is the configured Kraken iapi backoff state file, e.g.
  `kraken_iapi_backoff.json` under the app config directory.
- `checkpoint_rate` is intentionally separate from `tuned_rate`; code and docs
  must not describe a checkpoint restore as convergence.
- A 429 clears converged/tuned state by persisting `tuned_rate = 0.0` and the
  post-decrease live rate as `checkpoint_rate`.
- Legacy persistence files without AIMD fields decode with zero defaults and
  simply ramp from the configured cold rate.

## Consequences

- **Pro:** App restarts no longer throw away useful in-memory AIMD learning just
  because the rate had not been stable long enough to count as tuned.
- **Pro:** The limiter resumes near the last safe operating point while still
  adapting to current Cloudflare conditions.
- **Pro:** Known 429 ceilings survive restarts, reducing repeated excursions into
  already-discovered ban zones.
- **Pro:** Sync progress for Kraken Securities improves without increasing raw
  parallelism or ignoring provider pressure.
- **Con:** The persistence file now has two rate fields whose semantics must stay
  distinct: checkpoint is provisional; tuned is stable/converged.
- **Con:** AIMD cannot fix artificial backlog by itself. Scheduler demand shaping
  and fallback data sources remain necessary for full Kraken Securities coverage.

## Relationship to Other ADRs

- ADR-095 covers Kraken Spot public OHLC pacing and authenticated Spot REST
  account/history counters. This ADR covers the separate Kraken iapi surface used
  for Securities/equity-style data.
- ADR-099 covers Kraken WebSocket full-universe streaming. WebSocket streaming is
  the preferred path for live Spot OHLC updates; iapi AIMD governs REST-like
  Securities/equity metadata/history calls behind Cloudflare.

## Verification

- Unit tests cover checkpoint persistence, checkpoint restore without convergence,
  tuned-rate restore, 429 decrease persistence, discovered-ceiling persistence,
  and legacy-file decoding.
- `cargo test -p typhoon-engine iapi_limiter` exercises the limiter behavior.
- `cargo check -p typhoon-engine` and `cargo check -p typhoon-native` verify the
  engine/native integration compiles.
