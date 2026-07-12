# ADR-101: Kraken iapi AIMD Rate Discovery and Persistence

**Status:** Accepted (updated 2026-06-08 — measured ~6 req/s ceiling; see addendum) | **Date:** 2026-05-27

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
  to zero or ramp without limit. The default ceiling is a safety cap.
  **Caveat (2026-06-08): the measured real ceiling is ~6 req/s — see the addendum
  below before raising it.** `TYPHOON_KRAKEN_IAPI_AIMD_MAX_RATE` exists for
  controlled experiments only; raising it for throughput trips escalating 1015
  bans.

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

- Source: `typhoon-engine/src/broker/kraken/iapi_limiter.rs`.
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

## Update 2026-06-08: Measured iapi Ceiling (~6 req/s) and Concurrent-429 Coalescing

When native iapi history was temporarily widened to sweep the full ~12.7k
xStocks catalog (ADR-102), it drove the first sustained iapi load instead of the
prior demand-only trickle. ADR-112 subsequently reversed that scope and restored
iapi to demand-depth repair; the measured limiter findings remain valid:

- **The real Cloudflare ceiling for equity-history iapi is ~6–7 req/s, not "5+
  and probe higher."** An earlier session appeared to "ramp clean to 40 req/s,"
  but that was at trickle volume (a few calls per 10 s) where Cloudflare never
  actually saw 40. Under sustained catalog load, iapi returned HTTP 429 at
  ~11 req/s and `discovered_ceiling` settled near 6–7. **iapi is fundamentally a
  ~6 req/s lane.** Do **not** raise `aimd_max_rate` /
  `TYPHOON_KRAKEN_IAPI_AIMD_MAX_RATE` to "go faster" — it only trips escalating
  1015 bans (which hit the same IP used to trade Kraken). A full ~100k
  (symbol, tf) sweep is therefore *hours*; the only real speedup is fewer
  requests (timeframe derivation), not a higher cap.

- **A single overshoot must not compound into a multi-hour ban.** The token
  bucket lets a burst (up to `capacity`) fire before the multiplicative decrease
  lands, so several requests 429 within milliseconds. The escalation counter
  incremented once *per 429*, so one overshoot drove the window to
  `cloudflare_backoff_secs × 2^n` — a capped **1-hour ban**. `EscalationState`
  now records the armed deadline (`armed_until_unix`): while a backoff armed by a
  sibling of the same overshoot is still active, further 429s keep `consecutive`
  instead of compounding. Genuine repeat overshoots — only possible after the
  backoff expires, since `acquire()` short-circuits during it — still escalate
  one step each.

Config now reflects the measured edge: `aimd_max_rate` 10, `capacity` 4,
`aimd_increase_interval` 10 s, and the native equity-fetch concurrency
(`KRAKEN_EQUITIES_FETCH_PERMITS`, native side) 8 — sized to ~6 req/s × round-trip
so the bucket, not a wide permit pool, is the governor and an overshoot fires few
concurrent calls. An interim experiment (cap 120 / 48 permits / 3 s ramp) caused
exactly the 1-hour ban above and was reverted.

**Regression guard:** any future "make Kraken equities sync faster" work must not
raise the iapi rate cap or permit count past the ~6 req/s envelope. Reduce request
count (derive 4Hour/5Min/15Min/30Min from 1Hour/1Min) or lean harder on the
non-iapi breadth lanes (Alpaca/Yahoo, ADR-102) instead.

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
  engine-to-native integration compiles.
