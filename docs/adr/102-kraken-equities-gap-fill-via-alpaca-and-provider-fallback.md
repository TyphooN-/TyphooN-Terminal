# ADR-102: Kraken Equities Gap Fill via Alpaca and Provider Fallback

**Status:** Accepted / partially implemented (updated 2026-06-08 — native iapi now sweeps full catalog; see addendum) | **Date:** 2026-05-27

## Context

Kraken Securities / xStocks chart coverage is broad but uneven across intraday
and mid-timeframe bars. Sync Status can show the Kraken universe mostly healthy
at daily/weekly/monthly while `M15`, `M30`, `H1`, or `H4` stay stale or empty for
large parts of the same catalog. That is expected from the current source shape:
Kraken iapi is the native market-data source for tokenized equities, but it is
not guaranteed to expose complete historical bars for every timeframe and every
instrument.

Alpaca can often provide cleaner US-equity bars for the underlying ticker behind
an xStock symbol. For example, a Kraken equity wrapper symbol can usually be
mapped to the underlying US ticker by stripping Kraken wrapper suffixes such as
`.EQ` and normalizing the quote/pair wrapper. That makes Alpaca a useful fallback
for chart continuity, especially for `15Min` through `4Hour` where Kraken gaps
visibly damage MTF Grid charts and indicators.

The product goal is stricter than “fill the most obvious holes”: for Kraken
equities/xStocks, the terminal should build the deepest and freshest chart-usable
series it can for every enabled timeframe. Native Kraken remains authoritative,
but any compatible provider may prepend older bars or append fresher bars when
Kraken is empty, stale, shallow, or delayed. This is still provenance-tagged
assist data, not a rewrite of Kraken history.

The important caveat: Alpaca bars are not Kraken xStock bars. They represent the
underlying equity venue/feed, not the tokenized wrapper's actual Kraken trading
microstructure, weekend/holiday behavior, liquidity, or possible premium/discount.
Using them as silent replacement data would be wrong.

## Decision

Use Alpaca and later provider fallbacks as **provenance-tagged gap-fill bars** for
Kraken equities, not as a silent overwrite of Kraken data.

Make this the general broker-data policy, not a Kraken-only exception:

- Any enabled broker/source may assist another enabled broker/source when an
  explicit instrument-identity mapping says the histories are economically
  comparable enough for chart/research gap fill.
- The chart's selected broker remains authoritative for execution prices,
  account state, quote labels, order controls, and native-cache health.
- Assisted history is merged at read/render/research time with source provenance;
  it is never written into another source's native cache namespace.
- Same visible symbol text is only a candidate match, not proof of equivalence.
  The mapping layer must distinguish wrappers, CFDs, ADRs, FX suffixes, delayed
  feeds, quote currencies, and exchange-specific session calendars.
- New brokers should first reuse compatible existing history through this
  identity/provenance layer, then sync only genuinely missing/unsupported
  symbols and newer/older windows. That keeps broker onboarding from creating a
  massive avoidable sync-debt cliff.

Fallbacks must be controlled by an explicit **Settings → Backfill providers**
section. These switches are source-specific assist toggles, not broad broker
universe toggles:

- `CryptoCompare deep crypto history` — targeted Kraken Spot crypto prepend for
  USD/stablecoin-style pairs only; does not scan CryptoCompare as a universe.
- `Alpaca for all Kraken equities` — when enabled, every Kraken equities/xStocks
  candidate selected by the Kraken scheduler may also queue an Alpaca fetch for
  the same symbol/timeframe. This applies to the Kraken equities universe, not
  only held, charted, or watchlisted names, but it still follows Kraken's
  selected equity workset and must not trigger broad Alpaca-universe rotation.
- `Yahoo Chart fallback` — unkeyed equity/ETF fallback stored under
  `yahoo-chart:SYMBOL:TF`. Supports `1Min`, `5Min`, `15Min`, `30Min`, `1Hour`,
  `1Day`, `1Week`, and `1Month`, subject to Yahoo's range limits and symbol
  coverage. Dotted class-share symbols are requested using Yahoo's hyphen form
  (`BH.A` -> `BH-A`), but provider 404s for unresolved/SPAC/unit symbols are
  expected coverage gaps and are tombstoned rather than retried as app errors.

Alpaca fallback must also have an explicit **assist-only mode**. Connecting Alpaca
for Kraken gap fill must not automatically enable the normal broad Alpaca
universe sync. The terminal needs a settings-level switch that lets the user
connect Alpaca credentials while restricting Alpaca bar requests to Kraken
fallback jobs only.

Broker connectivity and bar-universe sync are separate controls. `Enable Alpaca`
plus `Connect Alpaca` authorizes account/trading calls and targeted fallback
fetches. The separate `Sync Alpaca universe bars` control is the only switch that
allows the scheduler to request the Alpaca asset catalog and rotate through the
full Alpaca equity bar universe. Leave it off for Kraken-equities assist.

### Source priority

For `kraken-equities:*` chart loads:

1. Native Kraken equity/iapi bars remain authoritative when present and fresh.
2. If an enabled timeframe from `1Min` through `4Hour` is empty, stale beyond the
   configured threshold, or shallower than a compatible fallback provider,
   enqueue fallback fetch for the mapped underlying ticker.
3. Store fallback bars under separate source namespaces:
   - `alpaca:SYMBOL:TF`
   - `yahoo-chart:SYMBOL:TF`
4. Build the chart series by loading the selected/authoritative source first,
   then gap-filling missing timestamps from alternate fallback namespaces.
5. Preserve a provenance mask/span list so UI, indicators, exports, and research
   packets can tell native Kraken bars from fallback underlying-equity bars.

Do not write Alpaca fallback bars into `kraken-equities:SYMBOL:TF`. That would
make the Sync Status lie and would erase the distinction between wrapper-market
prices and underlying-market prices.

## Timeframe and depth policy

Fallback providers are source-specific, but the merge objective is common:

- `front-fill`/freshness fill: append newer compatible bars when the native
  Kraken or Alpaca lane is delayed/gated. Both Kraken equities and the available
  Alpaca feed can be delayed; an additional source may still help if it is less
  stale for the symbol/timeframe, but it must be measured per response and shown
  as fallback provenance.
- `prepend`/deep-history fill: keep older compatible provider bars when the
  fallback source has more history than Kraken. Do not truncate the chart to the
  selected broker's native depth if an allowed fallback has earlier bars.
- `gap fill`: fill missing timestamps between native bars only when the fallback
  bar aligns to the same normalized timeframe/session policy.

Provider capabilities:

- Alpaca may fetch all standard enabled timeframes through the existing Alpaca
  bar path, subject to Alpaca feed/rate-limit/depth constraints. In the current
  account/feed posture it should be treated as delayed, not true real-time.
- Yahoo Chart may fetch all standard enabled timeframes, but Yahoo applies hard
  history windows: `1Min` is freshness-only, `5Min`/`15Min`/`30Min` are limited
  intraday history lanes, `1Hour` is a mid-depth lane, and daily/weekly/monthly
  are the useful deep-history lanes. Yahoo may be fresher than Kraken/Alpaca for
  some symbols, but it is not guaranteed real-time; compare latest bar timestamp
  before using it as front-fill. Yahoo coverage is not the Kraken equities
  catalog: many Kraken Securities symbols, especially SPAC/unit style tickers
  such as `.U`, may return HTTP 404 from Yahoo and must be treated as provider
  no-data.
- Kraken equities should not fetch `M1`/`M5` from iapi unless Kraken exposes a
  trustworthy lane for them. `M1`/`M5` chart usability should come from explicit
  fallback providers with provenance and timestamp freshness checks.

Implementation detail:

- Prefer direct provider bars for timeframes the provider natively supports.
- Materialize `4Hour` from `1Hour` bars by default when a provider lacks native
  `4Hour`; use `15Min` only if the aggregation code is session-aware and bounded.
- Allow fallback for `M1`/`M5` only through explicit provider toggles, per-source
  rate limits, no-data tombstones, and a freshness/depth comparison. These bars
  are high-volume, tier-sensitive, and prone to false precision, so they must
  stay out of native Kraken health totals.
- Daily/weekly/monthly should continue to prefer native Kraken iapi and existing
  high-timeframe sources, while retaining any older compatible fallback bars for
  chart/research history depth.

Merge selection rule: for each timestamp bucket, prefer native Kraken when
present. Outside native Kraken's covered window, include the deepest older and
freshest newer allowed fallback bars in provider-priority order. Never delete an
older fallback span merely because a shallower native source is selected.

## Symbol mapping

Kraken equity candidates should map to fallback tickers through a deterministic
normalizer:

1. Parse the cached key or Kraken pair metadata.
2. Strip Kraken market-data decorations (`kraken-equities:` prefix, timeframe
   suffix, `.EQ`, quote wrappers such as `USD`).
3. Normalize to an uppercase bare ticker.
4. Validate against the fallback provider before queueing:
   - Alpaca: asset exists and is data-eligible for the account/feed tier.
   - Optional future providers: Polygon, Yahoo chart, Nasdaq Data Link, or
     a paid equities feed, each with explicit coverage/rate-limit rules.
5. Persist failed mappings/no-data results as tombstones with expiry so the broad
   scheduler does not hammer symbols the provider cannot serve.

## Scheduler behavior

Fallback work is lower priority than user-visible Kraken chart freshness.
It is also lower priority than active-context SEC filing and news/article sync:
positions, orders, watchlist symbols, focused/open charts, and explicit user
scopes get research/event data before the broad Kraken equities fallback backlog
consumes provider budget.

### Alpaca assist-only setting

Add a settings checkbox/radio option under the Alpaca/data-source settings:

- `Alpaca full universe sync` — current behavior; sync Alpaca tradable universe
  according to the existing Alpaca scheduler.
- `Alpaca assist Kraken only` — connect/authenticate Alpaca, but suppress broad
  Alpaca bar sync. Alpaca bar requests may only be emitted by the Kraken equities
  fallback scheduler for mapped Kraken symbols/timeframes.
- `Alpaca disabled` — no Alpaca connection or bar requests.
- `Sync Alpaca universe bars` — explicit opt-in for normal broad Alpaca equity
  bar rotation. Off means Alpaca may still connect and serve targeted fallback
  requests, but the app must not fetch `GetAllAssets` for background Alpaca
  universe work or schedule `schedule_alpaca_pairs`.

The safe default for a newly connected Alpaca account should be **assist-only**
when the user enabled it from the Kraken gap-fill flow. Do not surprise the user
by starting a 100% Alpaca universe pull just because credentials became valid.

Implementation gates:

- Broad Alpaca sync queue checks `alpaca_full_bar_sync_enabled` before
  enumerating all assets.
- Kraken fallback queue checks `backfill_alpaca_kraken_equities_enabled` and only
  requests symbols produced by the Kraken equities scheduler.
- Yahoo fallback queue checks `backfill_yahoo_chart_enabled`, stores only under
  `yahoo-chart:*`, and uses independent pending/cooldown/tombstone state.
- If multiple fallback toggles are enabled, they may fetch the same
  symbol/timeframe in tandem. This is intentional for deep-history fill: dedup is
  done at chart merge time by timestamp while source provenance remains intact.
- UI copy must be blunt: `Alpaca for all Kraken equities: use Alpaca to fill
  Kraken equity chart gaps; do not sync the Alpaca universe.`
- Logs should show the mode on connect, e.g. `Alpaca connected (Kraken assist
  only — broad Alpaca sync disabled)`.
- LAN/server startup must preserve the setting so a headless restart does not
  silently revert to full Alpaca universe sync.

Recommended queue order:

1. Open/focused MTF Grid Kraken symbols and visible timeframes.
2. Stale, empty, shallow, or delayed `1Min` -> `4Hour` Kraken equity symbols
   currently visible or in watchlist/positions.
3. Broad Kraken equities fallback backlog.
4. Non-visible universe backfill.

The fallback scheduler must be bounded:

- separate provider rate limiter from Kraken iapi AIMD;
- per-source concurrency cap;
- no retry storm on no-data/permission errors;
- persistent progress cursor so restarts continue without rewalking the same
  thousands of symbols first;
- backpressure when the app is under heavy sync/render load.

Provider-assist throughput policy:

- Yahoo Chart uses the shared fallback HTTP client but is no longer executed
  inline in the broker command loop. Each Yahoo Chart request is spawned behind
  a small semaphore so a slow Yahoo response cannot block Kraken/Alpaca
  command handling, while the semaphore prevents unbounded full-catalog fanout.
- Alpaca broad stock assist should prefer `/v2/stocks/bars?symbols=...` batches
  for non-focused `15Min`, `30Min`, `1Hour`, `4Hour`, `1Day`, and `1Week`
  work. These are the Kraken-equities/provider-assist lanes where one request
  can resolve many symbols and materially improves catalog catch-up speed.
- Focused/visible symbols stay on the single-symbol Alpaca lane. Foreground work
  should not sit behind a large background batch, and single-symbol handling
  preserves the more careful incremental/backfill behavior used by active
  charts.
- `1Month` remains single-symbol/fallback-provider specific until an explicit
  monthly aggregation/provenance path exists. `1Min` and `5Min` remain demand
  scoped by policy; do not batch-broaden them just because an endpoint accepts
  the request.
- Batch results still write the `alpaca:*` cache namespace and emit per-symbol
  settlement/no-data/retry events. A batch transport optimization must not blur
  provenance or hide symbol-level failures.

## UI / observability

Users need to see that the chart is partially synthetic/fallback-filled.

Required indicators:

- Sync Status separates native Kraken coverage from fallback provider coverage.
  Alpaca and Yahoo appear as their own rows/totals when their assist
  toggles are enabled or when their cache namespaces contain bars. They must not
  be folded into the Kraken native percentage.
- Sync Status also shows `Merged` Kraken-equity rows/totals. `Merged` is the
  chart-usable coverage number: a symbol/timeframe is Healthy if any eligible
  source namespace has a healthy bar window (`kraken-equities`, Alpaca assist,
  Yahoo Chart). It is a derived coverage view, not a cache source,
  and is excluded from the auto-full-tilt native provider aggregate to avoid
  double-counting.
- The denominator is timeframe-specific and must stay explicit:
  - `1Day`, `1Week`, and `1Month` native Kraken rows use the full loaded
    Kraken equities catalog. Missing catalog symbols appear as empty expected
    rows so the window cannot claim full high-timeframe coverage while hundreds
    of catalog symbols are absent.
  - Native Kraken intraday rows remain demand-scoped unless/until iapi throughput
    and endpoint behavior prove that broader native intraday is safe.
  - Fallback provider rows use the full catalog for `15Min`, `30Min`, `1Hour`,
    `4Hour`, `1Day`, `1Week`, and `1Month` when that provider supports the
    timeframe. `1Min`/`5Min` stay demand/focus scoped until an explicit provider
    proves broad low-timeframe coverage is viable.
- Native `kraken-equities:*` rows remain separate from fallback rows; `Merged`
  only answers “is this chart-usable from any allowed source?”
- Chart status line shows the auto-resolved source, e.g. `Data: Auto → Kraken
  Equities`. Provider/fallback provenance belongs in Sync Status and future
  provenance-span metadata, not as a user-selectable chart-source picker. Auto
  detection should pick the best available source from broker/source priority and
  cache coverage.
- Optional subtle span coloring or hover metadata for fallback regions.
- Logs should say `Kraken gap-fill AAPL 1Hour via Alpaca: 240 bars` rather than
  pretending it was a Kraken fetch.

## Indicator policy

Indicators may consume merged series only when provenance is known.

Default policy:

- Use merged bars for visual chart continuity and broad technical indicators.
- Keep provenance available to strategies/backtests so they can reject fallback
  bars unless explicitly allowed.
- Research packets should disclose fallback coverage percentage for the analyzed
  window.
- Never use fallback-filled bars for execution-price assumptions on Kraken.



## Interaction with Tiered Scheduler (2026-06-10)

Gap-fill work via Alpaca and other providers is now also subject to the three-tier symbol priority (MTF Grid first). High-timeframe gap fills for focused symbols are preferred over low-timeframe background work.

## Consequences

### Pros

- MTF Grid charts become useful for Kraken equities even when iapi history is
  incomplete.
- Kraken iapi AIMD is protected from impossible backlog: missing bars that Kraken
  cannot serve can move to a separate fallback path instead of repeatedly taxing
  iapi.
- Users get clearer Sync Status semantics: native coverage, fallback coverage,
  and merged chart usability are different numbers.
- The architecture can support paid higher-quality providers later without
  changing the chart merge contract.

### Cons

- Fallback bars are economically different from Kraken wrapper bars.
- Alpaca account/feed tier affects depth and quality; IEX/free-tier data may be
  insufficient for some symbols/timeframes.
- Merge/provenance logic adds complexity to cache reads, Sync Status, and
  research exports.
- Incorrect symbol mapping can produce dangerous charts; mappings need validation
  and tombstones.

## Current implementation notes

- The Kraken equities catalog (`kraken_equity_universe_symbols`) is the authority
  for broad catalog coverage.
- The demand set is separate and intentionally narrower: held `.EQ` balances,
  watchlist `.EQ` entries, open/visible Kraken-equity charts, and legacy
  xStock-looking Kraken symbols.
- `schedule_kraken_equities_universe` uses the catalog set for native durable
  high timeframes and for fallback/provider-assist `15Min`+. Demand/focus paths
  may still queue lower-timeframe or native intraday fetches when a user opens a
  chart or watches/holds a symbol.
- Sync Status expected rows and `Merged` rows use the same timeframe policy, so
  the visible denominator matches the scheduler contract.

## Implementation status / reopen criteria

- The current implementation covers the native full-catalog denominator, Sync
  Status separation, bounded-concurrent Yahoo Chart fetches, Alpaca multi-symbol
  stock batches for broad non-focused assist work, Yahoo fallback fetchers,
  and assist-only controls. Reopen this ADR for code work when adding a new
  fallback provider, provenance-span rendering, strategy/backtest policy hooks,
  or the full depth/freshness merge policy described above. Any future change
  must keep the invariant above: high-TF full catalog, intraday demand-scoped
  unless an explicit provider policy says otherwise.

Historical implementation items that remain relevant as regression checks:

1. Normalized Kraken-equity-to-underlying mapping must handle `.EQ`, pair-name,
   display-name, and quote-wrapper cases.
2. Fallback cache namespaces must not overwrite native `kraken-equities:*` keys.
3. Provider no-data/permission results need tombstones so the broad scheduler does
   not hammer symbols a provider cannot serve.
4. Chart/research outputs must disclose fallback provenance when merged bars are
   used.
5. Tests should cover mapping, merge precedence, provider tombstones, and the
   high-TF catalog vs intraday demand denominator rule.

## Update 2026-06-08: Native iapi Sweeps the Full Catalog (as a slow depth-filler)

The earlier invariant — "native Kraken intraday stays demand-scoped; native
catalog only for durable high timeframes" — has been relaxed.
`kraken_equity_native_history_symbols` now returns the full catalog
(catalog-first, demand-fallback before the catalog has loaded), so native iapi
history sweeps every enabled timeframe, not just high-TF. The merged three-source
model is unchanged and remains the data-gathering contract:

> **Alpaca + Kraken iapi + Yahoo → Merged** (deduped by timestamp at read time;
> native Kraken authoritative where present; assist providers fill the rest).

Critically, widening native scope does **not** make native iapi a primary breadth
source. iapi is Cloudflare-hard-capped at ~6 req/s (ADR-101), so the full-catalog
native sweep is a **slow, AIMD-paced background depth-filler** that fills high
timeframes first (the workset selector is high-TF-first) and only reaches broad
native intraday after hours. **Alpaca and Yahoo remain the primary
breadth/intraday lanes** because they are not iapi-bound — they carry most of the
~12.7k universe, and the `Merged` column stays chart-usable without native iapi
ever being complete.

Regression checks added here:

1. Do **not** "speed up" native iapi by raising its rate or permit budget — it is
   a ~6 req/s wall (ADR-101). Widening native *scope* is fine; raising native
   *rate* is not.
2. A transient iapi HTTP 500 (`type: Internal error`, common per-symbol across a
   broad sweep) must apply a *per-symbol* cooldown, never a global
   equities-lane pause — one flaky symbol must not freeze the other ~12k. Only
   IP-wide 1015/429 warrants a global pause.
3. WS OHLC snapshots for the full xStocks subscription are requested only for the
   bounded high timeframes (1Hour–1Week) and live-only below, so the snapshot
   burst cannot OOM on the ~12k-symbol catalog. The WS path only serves the small
   set of genuinely WS-tradeable xStock tokens anyway; the broad catalog is an
   iapi/assist concern, not a WS one.

## Current policy / provider-gated design questions

Resolved policy:

- Broad Kraken-equities assist is opt-in. New sessions default Alpaca/Yahoo
  assist toggles off; enabling `Alpaca for all Kraken equities` applies to the
  Kraken equities scheduler workset without starting broad Alpaca universe sync.
- Yahoo Chart is the first non-Alpaca unkeyed fallback for equities/ETFs where
  Yahoo can resolve the symbol and timeframe. Dotted class shares use Yahoo's
  hyphenated request symbol; unresolved Yahoo 404/empty-result responses are
  durable provider no-data tombstones, not user-visible sync failures.

Provider-gated questions:

- Which additional provider should be the first dedicated front-fill lane for
  `1Min` -> `4Hour` when both Kraken and Alpaca are delayed/gated? Yahoo Chart is
  already available as an opportunistic lane, but it is not a contracted realtime
  feed.
- How visually loud should fallback spans be on charts? Too subtle hides risk;
  too loud makes charts unreadable.
