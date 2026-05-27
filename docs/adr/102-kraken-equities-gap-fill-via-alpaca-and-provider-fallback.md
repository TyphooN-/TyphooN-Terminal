# ADR-102: Kraken Equities Gap Fill via Alpaca and Provider Fallback

**Status:** Proposed
**Date:** 2026-05-27

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

Alpaca fallback must have an explicit **assist-only mode**. Connecting Alpaca for
Kraken gap fill must not automatically enable the normal broad Alpaca universe
sync. The terminal needs a settings-level switch that lets the user connect
Alpaca credentials while restricting Alpaca bar requests to Kraken fallback jobs
only.

### Source priority

For `kraken-equities:*` chart loads:

1. Native Kraken equity/iapi bars remain authoritative when present and fresh.
2. If a selected timeframe in `15Min`, `30Min`, `1Hour`, or `4Hour` is empty or
   stale beyond the configured threshold, enqueue fallback fetch for the mapped
   underlying ticker.
3. Store fallback bars under a separate source namespace, for example:
   - `kraken-equities-fill:SYMBOL:TF` for materialized fallback bars tied to the
     Kraken chart symbol; or
   - `alpaca-fill:SYMBOL:TF` if the same fallback should be shared outside Kraken
     chart routing.
4. Build the chart series by merging native Kraken spans first, then filling only
   missing intervals from fallback spans.
5. Preserve a provenance mask/span list so UI, indicators, exports, and research
   packets can tell native Kraken bars from fallback underlying-equity bars.

Do not write Alpaca fallback bars into `kraken-equities:SYMBOL:TF`. That would
make the Sync Status lie and would erase the distinction between wrapper-market
prices and underlying-market prices.

## Timeframe policy

Initial fallback scope should be limited to the timeframes the user called out:

- `15Min`
- `30Min`
- `1Hour`
- `4Hour`

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
- Kraken fallback queue checks `alpaca_kraken_assist_enabled` and only requests
  symbols produced by the Kraken mapping table.
- UI copy must be blunt: `Assist Kraken only: use Alpaca to fill Kraken equity
  chart gaps; do not sync the Alpaca universe.`
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

- Sync Status separates native Kraken coverage from fallback-filled coverage.
  Example columns: `Native`, `Fallback`, `Merged`, `Stale`.
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

## Implementation plan

1. Add a normalized Kraken-equity-to-underlying mapper with unit tests for `.EQ`,
   pair-name, display-name, and quote-wrapper cases.
2. Add a fallback cache namespace and metadata schema:
   - source provider;
   - underlying symbol;
   - original Kraken symbol;
   - timeframe;
   - fetch timestamp;
   - no-data/permission tombstones.
3. Add a merge reader that returns `(bars, provenance_spans)` without changing
   the existing raw `kraken-equities:*` cache contract.
4. Add scheduler tasks only for `15Min`, `30Min`, `1Hour`, and `4Hour`.
5. Wire Alpaca as provider 1, using existing ADR-087 rate/tier logic and
   no-data tombstones. Gate this behind the Alpaca assist-only/full-sync setting
   so connecting Alpaca for Kraken help does not start the broad Alpaca universe
   pull.
6. Update Sync Status to show native/fallback/merged coverage separately.
7. Update chart status/source badges and research packet disclosure.
8. Add tests:
   - mapping tests;
   - merge precedence tests;
   - no overwrite of native Kraken keys;
   - fallback bars fill only missing intervals;
   - no-data tombstone suppresses repeated fetches;
   - chart load prefers native bars when both exist.

## Open questions

- Should fallback be enabled by default for all Kraken equities, or only for
  watchlist/position/visible symbols until provider load is understood?
- Should `4Hour` be materialized from `1Hour` bars or from `15Min` bars for
  better session-boundary control?
- Which non-Alpaca provider should be the second fallback for symbols Alpaca
  cannot serve or accounts without sufficient data tier?
- How visually loud should fallback spans be on charts? Too subtle hides risk;
  too loud makes charts unreadable.
