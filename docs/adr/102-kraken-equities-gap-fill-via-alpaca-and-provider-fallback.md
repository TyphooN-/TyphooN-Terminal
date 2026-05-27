# ADR-102: Kraken Equities Gap Fill via Alpaca and Provider Fallback

**Status:** Accepted / partially implemented | **Date:** 2026-05-27

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
- `Stooq daily fallback` — unkeyed daily equity fallback stored under
  `stooq:SYMBOL:1Day`. It is daily-only by design; it must not fabricate
  intraday coverage. Stooq availability is local-network/IP dependent: a third-
  party status page may show Stooq up while this machine cannot browse or fetch
  `stooq.com`. On transport/provider failure, the app pauses the optional Stooq
  assist lane instead of logging one failure per Kraken equity symbol.

Alpaca fallback must also have an explicit **assist-only mode**. Connecting Alpaca
for Kraken gap fill must not automatically enable the normal broad Alpaca
universe sync. The terminal needs a settings-level switch that lets the user
connect Alpaca credentials while restricting Alpaca bar requests to Kraken
fallback jobs only.

### Source priority

For `kraken-equities:*` chart loads:

1. Native Kraken equity/iapi bars remain authoritative when present and fresh.
2. If a selected timeframe in `15Min`, `30Min`, `1Hour`, or `4Hour` is empty or
   stale beyond the configured threshold, enqueue fallback fetch for the mapped
   underlying ticker.
3. Store fallback bars under separate source namespaces:
   - `alpaca:SYMBOL:TF`
   - `yahoo-chart:SYMBOL:TF`
   - `stooq:SYMBOL:1Day`
4. Build the chart series by loading the selected/authoritative source first,
   then gap-filling missing timestamps from alternate fallback namespaces.
5. Preserve a provenance mask/span list so UI, indicators, exports, and research
   packets can tell native Kraken bars from fallback underlying-equity bars.

Do not write Alpaca fallback bars into `kraken-equities:SYMBOL:TF`. That would
make the Sync Status lie and would erase the distinction between wrapper-market
prices and underlying-market prices.

## Timeframe policy

Fallback providers are source-specific:

- Alpaca may fetch all standard enabled timeframes through the existing Alpaca
  bar path, subject to Alpaca feed/rate-limit/depth constraints.
- Yahoo Chart may fetch all standard enabled timeframes, but Yahoo applies hard
  history windows: `1Min` is freshness-only, lower intraday is limited, and
  higher timeframes are the useful deep-history lane. Yahoo coverage is not the
  Kraken equities catalog: many Kraken Securities symbols, especially SPAC/unit
  style tickers such as `.U`, may return HTTP 404 from Yahoo and must be treated
  as provider no-data.
- Stooq is `1Day` only. Do not use it for `1Week`/`1Month` unless a separate
  aggregation/provenance step is added, and never use it for intraday. Treat
  connection failures as provider unavailable from this machine; pause the Stooq
  lane and show degraded assist state instead of continuing a broad retry storm.
- Kraken equities still never fetch `M1`/`M5` from iapi because the feed is
  delayed and those bars imply false precision.

Implementation detail:

- Prefer direct Alpaca bars for timeframes Alpaca natively supports.
- Materialize `4Hour` from lower-timeframe Alpaca bars if Alpaca does not expose
  a direct 4h endpoint shape compatible with the current cache writer.
- Do not use fallback for `M1`/`M5` until the app has an explicit low-timeframe
  policy. Those are high-volume, tier-sensitive, and prone to false precision.
- Daily/weekly/monthly should continue to prefer native Kraken iapi and existing
  high-timeframe sources unless there is a separate history-depth decision.

## Symbol mapping

Kraken equity candidates should map to fallback tickers through a deterministic
normalizer:

1. Parse the cached key or Kraken pair metadata.
2. Strip Kraken market-data decorations (`kraken-equities:` prefix, timeframe
   suffix, `.EQ`, quote wrappers such as `USD`).
3. Normalize to an uppercase bare ticker.
4. Validate against the fallback provider before queueing:
   - Alpaca: asset exists and is data-eligible for the account/feed tier.
   - Optional future providers: Polygon, Stooq, Yahoo chart, Nasdaq Data Link, or
     a paid equities feed, each with explicit coverage/rate-limit rules.
5. Persist failed mappings/no-data results as tombstones with expiry so the broad
   scheduler does not hammer symbols the provider cannot serve.

## Scheduler behavior

Fallback work is lower priority than user-visible Kraken chart freshness.

### Alpaca assist-only setting

Add a settings checkbox/radio option under the Alpaca/data-source settings:

- `Alpaca full universe sync` — current behavior; sync Alpaca tradable universe
  according to the existing Alpaca scheduler.
- `Alpaca assist Kraken only` — connect/authenticate Alpaca, but suppress broad
  Alpaca bar sync. Alpaca bar requests may only be emitted by the Kraken equities
  fallback scheduler for mapped Kraken symbols/timeframes.
- `Alpaca disabled` — no Alpaca connection or bar requests.

The safe default for a newly connected Alpaca account should be **assist-only**
when the user enabled it from the Kraken gap-fill flow. Do not surprise the user
by starting a 100% Alpaca universe pull just because credentials became valid.

Implementation gates:

- Broad Alpaca sync queue checks `alpaca_full_universe_sync_enabled` before
  enumerating all assets.
- Kraken fallback queue checks `backfill_alpaca_kraken_equities_enabled` and only
  requests symbols produced by the Kraken equities scheduler.
- Yahoo fallback queue checks `backfill_yahoo_chart_enabled`, stores only under
  `yahoo-chart:*`, and uses independent pending/cooldown/tombstone state.
- Stooq fallback queue checks `backfill_stooq_daily_enabled`, only accepts
  `1Day`, stores only under `stooq:*`, and uses independent pending/cooldown/
  tombstone state.
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
2. Stale/empty `15Min` -> `4Hour` Kraken equity symbols currently visible or in
   watchlist/positions.
3. Broad Kraken equities fallback backlog.
4. Non-visible universe backfill.

The fallback scheduler must be bounded:

- separate provider rate limiter from Kraken iapi AIMD;
- per-source concurrency cap;
- no retry storm on no-data/permission errors;
- persistent progress cursor so restarts continue without rewalking the same
  thousands of symbols first;
- backpressure when the app is under heavy sync/render load.

## UI / observability

Users need to see that the chart is partially synthetic/fallback-filled.

Required indicators:

- Sync Status separates native Kraken coverage from fallback provider coverage.
  Alpaca, Yahoo, and Stooq appear as their own rows/totals when their assist
  toggles are enabled or when their cache namespaces contain bars. They must not
  be folded into the Kraken native percentage.
- Sync Status also shows `Merged` Kraken-equity rows/totals. `Merged` is the
  chart-usable coverage number: a symbol/timeframe is Healthy if any eligible
  source namespace has a healthy bar window (`kraken-equities`, Alpaca assist,
  Yahoo Chart, or Stooq daily). It is a derived coverage view, not a cache source,
  and is excluded from the auto-full-tilt native/provider aggregate to avoid
  double-counting.
- The denominator is timeframe-specific and must stay explicit:
  - `1Day`, `1Week`, and `1Month` use the full loaded Kraken equities catalog.
    Missing catalog symbols appear as empty expected rows so the window cannot
    claim full high-timeframe coverage while hundreds of catalog symbols are
    absent.
  - Intraday rows use the demand set only: positions, watchlist, open/visible
    charts, and legacy xStock-looking Kraken symbols. The app must not fabricate
    a 12k-symbol native iapi intraday backlog.
  - Fallback provider rows follow the same denominator rule: full catalog for
    durable high timeframes, demand set for intraday provider work.
- Native `kraken-equities:*` rows remain separate from fallback rows; `Merged`
  only answers “is this chart-usable from any allowed source?”
- Chart status line shows a concise source badge, e.g. `Data: Kraken Equities +
  Alpaca gap-fill`.
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
  for broad durable coverage on `1Day`, `1Week`, and `1Month`.
- The demand set is separate and intentionally narrower: held `.EQ` balances,
  watchlist `.EQ` entries, open/visible Kraken-equity charts, and legacy
  xStock-looking Kraken symbols.
- `schedule_kraken_equities_universe` uses the catalog set only for durable high
  timeframes. Demand/focus paths may still queue intraday fetches when a user
  opens a chart or watches/holds a symbol.
- Sync Status expected rows and `Merged` rows use the same timeframe policy, so
  the visible denominator matches the scheduler contract.

## Implementation plan / reopen criteria

The current implementation covers the native/full-catalog denominator and the
Sync Status separation. Reopen this ADR for code work when adding a new fallback
provider, provenance-span rendering, or strategy/backtest policy hooks. Any
future change must keep the invariant above: high-TF full catalog, intraday
demand-scoped unless an explicit provider policy says otherwise.

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

## Current policy / remaining reopen questions

Resolved policy:

- Broad Kraken-equities assist is opt-in. New sessions default Alpaca/Yahoo/Stooq
  assist toggles off; enabling `Alpaca for all Kraken equities` applies to the
  Kraken equities scheduler workset without starting broad Alpaca universe sync.
- Yahoo Chart is the first non-Alpaca unkeyed fallback for equities/ETFs where
  Yahoo can resolve the symbol and timeframe. Dotted class shares use Yahoo's
  hyphenated request symbol; unresolved Yahoo 404/empty-result responses are
  durable provider no-data tombstones, not user-visible sync failures.
- Stooq is the second fallback for daily equity history only. It must not be
  counted as weekly/monthly coverage until a separate aggregation/provenance pass
  exists.

Remaining reopen questions:

- Should `4Hour` be materialized from `1Hour` bars or from `15Min` bars for
  better session-boundary control?
- How visually loud should fallback spans be on charts? Too subtle hides risk;
  too loud makes charts unreadable.
