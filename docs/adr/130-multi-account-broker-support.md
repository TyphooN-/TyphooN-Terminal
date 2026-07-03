# ADR-130: Multi-Account Broker Support (Pooled Sync Fan-Out, Account-Primary Cycling, TradeCopy)

**Status:** Accepted / Implemented (2026-07-02)
**Date:** 2026-07-02

## Context

Each broker module previously held exactly one login. That wasted free capacity
and blocked two workflows:

1. **Historical sync throughput.** Alpaca's free tier allows **1 live + 3 paper
   accounts**, and the market-data rate limit (Basic: 200 req/min) is enforced
   **per account key**. With one account, the broad Alpaca/Merged universe
   (~12.4k symbols × 6 assist timeframes) catches up at single-account speed;
   the Sync Status screenshot that motivated this ADR showed Merged 15Min–4Hour
   stuck at ~0.7%. Four accounts ≈ 4× the aggregate historical budget for free.
2. **Account workflows.** Testing strategies across paper accounts, then
   promoting to live, needs (a) fast switching of which account the terminal
   trades/reads, and (b) copying positions/orders between accounts.

Kraken market data is public (WS v2 + public REST need no key), so extra Kraken
accounts add **no** sync capacity — but multiple Kraken logins are still useful
as order-routing identities. The Kraken iapi depth lane stays demand-scoped and
rate-walled regardless (see ADR-112/-128 — do not try to make iapi fast).

## Decision

### 1. Account pools in the broker runtime

`typhoon-broker-runtime/src/account_pool.rs`:

- `AlpacaAccountPool` — owns one `AlpacaBroker` **per account** (each with its
  own `RateLimiter`s), a `primary_idx`, an atomic round-robin `data_cursor`,
  the live-mirroring flag, and the explicit mirror-target opt-in set. The
  protocol roles per account (`BrokerAccountSpec`): `trade_enabled`
  (TradeCopy/mirror eligibility) and `data_sync_enabled` (bar-sync rotation
  member). Since 2026-07-03 the native UI no longer exposes per-slot toggles —
  every configured Alpaca slot is sent with both flags on, so all slots behave
  identically (see Update below).
- `KrakenAccountPool` — trading identities only; holds the WS-token override
  for account 1.

`BrokerCmd::Connect` now carries `accounts: Vec<BrokerAccountSpec>` +
`primary_id` and validates all accounts concurrently; `KrakenConnect` gains
`extra_accounts`. The runtime reports `BrokerMsg::AccountRoster { broker,
accounts }` (per-account connect state, equity, primary flag) after connect
and after every primary switch.

**Routing rules:**

- *Trading / account data / streams* → the **primary** account only.
  `SetPrimaryAccount` re-points it at runtime, re-emits account state
  (account, positions, open orders, recent fills), and **aborts + restarts the
  Alpaca trade-updates WS** so the old account's fills stop overwriting the
  new account's state. (Kraken caveat: the private ownTrades/openOrders WS
  still follows the previous account until restart; surfaced as a log line.)
- *Historical bar fetches* (`AlpacaFetchBars`, `AlpacaFetchBarsBatch`,
  `FetchAllBars`) → `pool.next_data_broker()`, round-robin over data-sync
  accounts. The scheduler assigns each (symbol, TF) task once; accounts only
  decide *which key executes it*, so there is no duplicate-write hazard — all
  accounts feed the same `alpaca:SYM:TF` cache keys.

### 2. Aggregate capacity scaling (native scheduler)

- `alpaca_aggregate_historical_rpm()` = per-account RPM × data-account count.
- `alpaca_sync_capacity()` tiers `queue_window`/`batch_size` by the aggregate
  RPM and multiplies `fetch_permits` by the pool size (capped at 48); each
  in-flight worker is still paced by its own account's limiter, so no account
  exceeds its individual budget. `ConfigureAlpacaSync` applies the per-account
  RPM hint to **every** pool member.

### 3. Credentials & persistence (native)

- Keyring slots: slot 1 keeps the legacy `alpaca_api_key`/`alpaca_secret`
  (backward compatible); slots 2–4 use `alpaca_api_key_N`/`alpaca_secret_N`
  and `kraken_api_key_N`/`kraken_api_secret_N`, with the same SQLite `cred:`
  fallback.
- Settings → API Keys renders the four Alpaca slots **identically**: Key,
  Secret, and a Paper/Live mode choice per slot — no per-slot label or
  Trade/Data checkboxes. Kraken slots 2–4 are Key/Secret only (identities).
  Account labels are auto-derived (`Alpaca 2 (Paper)`, `Kraken 3`).
- Credential fields persist to the keyring (+ SQLite `cred:` fallback)
  **as soon as the field is edited** (`persist_credential_async`, off the
  render thread); clearing a field deletes the stored entry. The Connect
  click and the explicit quit sweep remain as belt-and-braces. Previously
  slot keys were only written inside a successful Connect click — keys typed
  while already connected were silently lost on an unclean exit.
- The per-slot Paper/Live mode and the per-broker primary account id persist
  in session sync-preferences; secrets never leave the keyring
  (`#[serde(skip)]` on the credential fields).

### 4. Primary cycling (top bar)

The `Primary:` chip now cycles **(broker, account)** pairs — every connected
account of every enabled broker (e.g. `Alpaca · Live → Alpaca · Paper 1 → … →
Kraken`). Broker-level effects (order routing default, ADR-126 merge trusted
lane) fire only when the *broker* changes; an intra-broker account change sends
`SetPrimaryAccount` only. The chip renders whenever ≥2 brokers **or** ≥2
accounts exist. The toolbar/account panels show the **primary account's**
paper/live mode (`alpaca_primary_is_paper()`), not the slot-1 flag.

### 5. TradeCopy (`TRADECOPY` console command or Trading → TradeCopy…)

Two modes, Alpaca-first (testable on paper accounts; Kraken copy is future
work — no paper environment, spot-balance semantics differ):

- **One-shot position copy** (`BrokerCmd::AlpacaTradeCopy`): fetch source +
  target positions, compute per-symbol **signed qty deltas**
  (`trade_copy_deltas`, unit-tested; shorts negative), submit market orders on
  each target; optional *flatten extra* closes target symbols the source
  doesn't hold. Results stream to the Log as `TradeCopy → <account>` lines.
- **Live mirroring** (`BrokerCmd::SetOrderMirroring { enabled, target_ids }`):
  while on, every app-placed Alpaca order (market/notional/limit/stop/
  stop-limit/bracket/OCO/trailing, position closes, exit syncs) is replicated
  to **each explicitly checked target account**, tagged `[mirror → <account>]`.
  Cancels/modifies are **not** replicated (order ids are account-specific;
  a log note says so).

Trade copying is **strictly opt-in, never opt-out**: the mirror toggle stays
disabled until at least one target account is checked, the runtime refuses to
enable mirroring with an empty target set, unchecking the last target
auto-disables mirroring, and neither the flag nor the target set persists
across restarts — every session starts with copying OFF. Additional rails:
**live** accounts are locked out as targets unless "Allow LIVE accounts as
targets" is explicitly checked; disconnected targets are skipped with errors.

### 6. Sync Status honesty for disabled timeframes

Rows whose timeframe is unchecked in *Enabled Sync TFs* (e.g. M1/M5) are now
dropped from the Sync Status window **and** from the broker/overall
percentages (`BarSyncInputs::compute` filters on the enabled set). Automated
sync already skipped those TFs; counting their leftover cached rows dragged
the % down and could pin auto-full-tilt on work the scheduler is told to
ignore.

### 7. Recent-fills pipeline fix (chart arrows + panel)

Alpaca fills were fetched **once at connect**, so fills landing mid-session
never reached the Recent Fills panel or chart buy/sell arrows; additionally
the chart-arrow timestamp parser only accepted `…Z`-suffixed times, silently
dropping RFC3339 offset forms (`-04:00`) → `ts=0` → no arrow. Now:

- the trade-updates WS refresh re-pulls FILL activities (shared
  `fetch_and_send_recent_fills`),
- the 30 s positions/orders reconcile also refreshes activities (safety net
  while the WS is down; the raw payload stays log-suppressed),
- primary-account switches re-pull fills (and an empty result **clears** stale
  fills from the previous account),
- the arrow parser tries `DateTime::parse_from_rfc3339` first.

## Sync-architecture audit (async / O(1) / high-TF-first gap fill)

Requested alongside this change; findings (no scheduler rewrite needed):

- **High-TF-first is strict**: `select_alpaca_sync_workset_rotating*` walks
  MN1→…→M1 and spends the whole refill batch on the highest timeframe with
  actionable work (Missing → Stale → Backfill buckets, focus-first within a
  bucket) — matching the merged-universe goal.
- **O(1) hot paths**: per-(symbol,TF) sync state comes from
  `BgData::source_sync_state`, built in a single off-thread pass
  (`build_source_sync_state_maps`); scheduler lookups are hash hits. The Sync
  Status matrix scan and cold chart loads already run on blocking workers
  (ADR-128 lineage).
- **Amortized scanning**: the rotating cursor scans bounded windows
  (`background_scan_limit`) and early-exits on the first window with work, so
  steady-state refills are O(window), not O(catalog).
- **Async fan-out**: bar fetches are spawned tasks gated by a semaphore
  (`fetch_permits`) + per-account limiters; multi-account fan-out slots in at
  the dispatch point without touching scheduler logic.
- **Broker modularity**: assist lanes remain capability-driven (ADR-126/-129);
  pools are per-broker constructs behind the same `BrokerCmd` seam, so a
  future broker adds a pool + capability flags, not new UI forks.

## Consequences

- Free-tier Alpaca setups can roughly **4×** broad historical sync throughput
  (1 live + 3 paper), directly shrinking the Merged-universe catch-up window.
- The primary switch is now account-granular; ADR-126 semantics (primary
  broker = routing default + trusted merge lane) are unchanged at the broker
  level.
- TradeCopy centralizes cross-account replication in the runtime; the UI only
  builds requests. Copy correctness is bounded by market-order fills (no
  limit-price preservation in the one-shot copy — deltas are qty-based).
- Kraken multi-account is identity-only for now; private-WS follow-on-switch
  and Kraken TradeCopy are explicitly deferred (no paper environment,
  spot-balance semantics differ — blocked on a product decision, not on
  plumbing; the pool seam is already in place).

## Update (2026-07-03) — uniform slots, on-edit keyring persistence, opt-in mirroring

User-driven comb-over of the multi-account settings surface:

- **Uniform Alpaca slots.** Slots 1–4 render identically (Key / Secret /
  Paper|Live). The per-slot Label, Trade, and Data controls were removed;
  `ExtraAccountConfig` shrank to `{api_key, secret, paper}` and specs always
  ship `trade_enabled = data_sync_enabled = true`. Four accounts total —
  slot 1 (legacy keyring names) plus slots 2–4.
- **Keyring persistence bug fixed.** Slot credentials were only stored inside
  the "Connect Alpaca" click handler, which is a no-op while already
  connected — extra-account keys entered mid-session were lost on an unclean
  exit. All credential fields (Alpaca 1–4, Kraken main REST/WS pair, Kraken
  slots 2–4) now persist on field edit via `persist_credential_async`.
- **TradeCopy from the console.** `TRADECOPY` / `TRADE_COPY` / `COPYTRADE`
  opens the TradeCopy window; the Trading-menu entry remains.
- **Opt-in mirroring.** `SetOrderMirroring` now carries the explicit
  `target_ids` opt-in set (see §5); mirroring can never fan out to accounts
  that were not individually checked, and it always starts disabled.
