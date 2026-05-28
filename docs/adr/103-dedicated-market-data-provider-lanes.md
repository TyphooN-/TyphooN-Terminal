# ADR-103: Dedicated Market-Data Provider Lanes for Deep/Fresh Bars

**Status:** Proposed | **Date:** 2026-05-28

## Context

Kraken Securities / xStocks native bars are authoritative for Kraken wrapper-market
history, but they are incomplete and delayed for broad intraday use. Alpaca assist
helps, but the currently available Alpaca posture is also delayed/gated for some
US-equity data. For `1Min` through `4Hour`, the terminal needs the deepest and
freshest chart-usable series it can obtain without making the product unusable for
people with limited resources.

This ADR evaluates dedicated market-data provider lanes: providers that can supply
US-equity bars, quotes, bid/ask, and eventually L2/order-book context independent
of the selected execution broker.

The goal is not to turn TyphooN into a data-vendor product. The goal is:

1. Work well with no paid subscriptions where legally and technically reasonable.
2. Let users bring their own free/cheap keys or broker entitlements.
3. Preserve source provenance so charts do not lie.
4. Avoid hammering Kraken iapi or broker APIs for data they are bad at providing.
5. Keep execution/broker state separate from chart history enrichment.

Pricing and plan details below are a point-in-time investigation from 2026-05-28.
Provider plans change. Treat this as implementation guidance, not a contract.

## Decision direction

Use a layered provider model:

1. **Native broker/source lane**
   - Kraken equities, Kraken Spot, MT5/Darwinex, Alpaca, tastytrade, etc.
   - Authoritative for execution source, account state, native quotes, fills,
     positions, and source-specific health.

2. **Zero-key best-effort lane**
   - Yahoo Chart for intraday/daily bars where available.
   - Stooq for daily history only where reachable.
   - SEC/NasdaqTrader/reference feeds for metadata, not OHLC.
   - This lane must be optional, rate-limited, cached, and honestly labeled as
     unofficial/best-effort where applicable.

3. **User-key free/cheap lane**
   - Alpaca free/IEX, Tiingo, Twelve Data, Alpha Vantage free, FMP free/EOD,
     MarketData.app free, Tradier/IBKR/Schwab broker-authenticated feeds.
   - User brings credentials/entitlements.
   - These lanes are more legally defensible than scraping anonymous web data.

4. **Paid professional lane**
   - Polygon/Massive, Alpaca paid, FMP paid, MarketData.app paid, Intrinio,
     Nasdaq Data Link/official exchange products.
   - Optional. Never required for baseline product usefulness.

The chart merge engine should consume all enabled compatible lanes with
provenance:

- Native source wins on overlapping timestamp buckets.
- Older fallback bars may prepend deep history.
- Newer fallback bars may front-fill if their latest timestamp beats the selected
  native/source lane.
- Fallback bars never overwrite native cache namespaces.
- Indicators/research may use merged bars only when provenance is available.
- Strategies/backtests must be able to reject fallback spans unless explicitly
  allowed.

## Important clarification: bars vs live market microstructure

A dedicated market-data provider can reduce or replace a lot of historical bar
sync work if it provides better, cheaper, deeper, or fresher bars than the broker
source. That does **not** eliminate broker/native responsibilities:

- Execution brokers still own order placement, fills, positions, balances,
  account state, and order status.
- The chart still needs native broker overlays: fills, positions, average price,
  P/L, bracket orders, stops/targets, and broker-specific trading state.
- If bid/ask is available from a WebSocket, it should be painted onto the current
  chart regardless of which historical bar provider built the candle series.
- Buy/sell UI should show best available bid/ask/spread/staleness before a market
  order. A “Market Buy” button without live spread/staleness is blind.
- L2/order-book data is a separate lane from historical OHLC bars. It should feed
  DOM/book widgets, spread/depth badges, slippage estimates, and order-entry risk,
  not mutate historical candles.

So a paid/free provider can become the primary **bar lane**, but it should not
replace broker-native **execution state** or live quote/depth overlays.

## Provider comparison

### Zero-key / 100% free-ish options

There is no clean, official, zero-key, free, realtime, consolidated US-equity
OHLC + quote + L2 feed suitable for redistribution. Anyone claiming otherwise is
usually hiding one of these problems: unofficial endpoint, delayed data,
exchange-only data, no redistribution rights, tiny quota, no intraday history, or
requires a broker account.

| Provider | Cost | Key/account | Useful for | Freshness/depth | Pros | Cons / risk |
|---|---:|---|---|---|---|---|
| Yahoo Chart | $0 | No key | Intraday + daily bars | Observed: `1m` up to ~7-8d, `5m/15m/30m` ~60d, `1h` ~730d, daily long range | Best zero-key UX; broad coverage; already implemented as `yahoo-chart:*` | Unofficial for this use, ToS/rate-limit risk, can change/break, not guaranteed realtime, no redistribution confidence |
| Stooq | $0 | No key | Daily OHLCV | Daily history; endpoint availability can vary by network/IP | Lightweight CSV; good deep daily fallback | Daily only; current reachability can fail; licensing/redistribution unclear |
| IEX HIST | $0 | No retail API key for downloads | Official IEX exchange historical feed | T+1, IEX-only, feed/PCAP-oriented | Legally cleaner; official exchange-specific history | Not consolidated, not realtime, not ready-made chart bars, requires feed processing |
| SEC EDGAR | $0 | User-Agent | Filings/fundamentals/events | Realtime-ish filings, not prices | Official and free | Not OHLC/quote market data |
| NasdaqTrader files | $0 | No key | Symbol/reference/regulatory metadata | Daily/current files | Official symbol/ref data | Not chart bars or live quotes |

Conclusion: Yahoo Chart + Stooq is the only practical zero-key chart fallback pair,
but it must be presented as best-effort/unofficial and disabled/kill-switchable if
it starts failing. It is useful for low-resource users, but it is not a licensed
market-data foundation.

### User-key free / cheap developer options

| Provider | Free tier | Cheapest useful paid tier seen | Intraday/history | Freshness | Pros | Cons / gotchas |
|---|---:|---:|---|---|---|---|
| Alpaca Market Data | Yes | Algo Trader Plus around $99/mo | Free: 7+ years history, IEX-focused; paid covers all US exchanges | Free IEX realtime via websocket; broader/API data gated/delayed | Already integrated; legitimate; 200 req/min free; good user-key lane | Requires Alpaca account/key; IEX-only free is not SIP/NBBO; paid is not cheap for limited-resource users |
| Tiingo | Yes | Power around $30/mo | Intraday OHLC via IEX endpoints; exact intraday depth needs verification | IEX/Tiingo reference realtime caveats | Cheap; generous paid quotas; good API | IEX-only/reference-price caveats; entitlement terms; not full-market tape |
| Twelve Data | Yes | Grow shown as from ~$29/mo or higher depending billing/page | Supports 1m, 5m, 15m, 30m, 1h, 2h, 4h, daily+; outputsize 5000 | Claims realtime US equities on free/paid | Good interval coverage including 4h; easy API | Free quota small; pricing display inconsistent; historical depth must be measured per symbol/interval; non-commercial/internal-display limits |
| Alpha Vantage | Yes, 25 req/day | Around $49.99/mo | 1m/5m/15m/30m/60m; docs claim 20+ years intraday by month | Historical by default; realtime/delayed entitlement separate | Excellent deep intraday history shape | Free quota too tiny for broad backfill; paid more expensive than several alternatives; entitlement friction |
| Finnhub | Yes for other endpoints | Public all-in-one around $3500/mo | Stock candles are premium; 1/5/15/30/60/D/W/M | Premium | Already used for research/news | Not viable for low-cost intraday bars |
| MarketData.app | Yes | Starter around $12/mo annual / $30 monthly; Trader around $30/mo annual / $75 monthly | Historical candles including 1-minute/hourly/daily; free 1 year, Starter 5 years, Trader unlimited | Stocks API quotes listed as delayed except some midpoint products | Very cheap for historical bars; simple pricing | Freshness/front-fill may still be delayed; verify live quote semantics before relying on market orders |
| Tradier | Brokerage Lite $0 | Account-based | Time & Sales tick/1m/5m/15m; short history windows | Realtime for brokerage account; sandbox delayed | Cheap for users with Tradier; useful live quote lane | Requires brokerage account/OAuth; intraday history shallow; 120 req/min production |
| IBKR | Account-based | Account/data entitlement based | Historical + live via broker APIs | Free non-consolidated Cboe One/IEX for clients; delayed elsewhere | Powerful for users who already have IBKR | Heavy login/session/API complexity; not anonymous; entitlements vary |
| FMP | Free EOD only | Premium around $49/mo; Ultimate around $99/mo for explicit 1-minute | Intraday chart APIs: 1m/5m/15m/30m/1h/4h; paid depth up to 30y/full | Pricing claims realtime on paid | Good endpoint coverage, including 4h | Free does not solve intraday; 1m appears tied to higher paid tier |

### Paid/professional options

| Provider | Cheapest relevant public tier seen | What it buys | Pros | Cons |
|---|---:|---|---|---|
| Polygon / Massive | Stocks Starter around $29/mo; Developer around $79/mo; Advanced around $199/mo | Starter: delayed aggregates, 5y, unlimited calls; Developer: 10y; Advanced: realtime/all-history | Strongest low-cost professional-ish bar lane; great aggregates; easy implementation | Realtime costs more; individual/non-pro terms; still not “free for everyone” |
| Intrinio | Thousands/year | Cboe/IEX/Nasdaq/stock tick history products | Professional licensing and data quality | Too expensive for this product's low-resource goal |
| Nasdaq Data Link / official exchange feeds | Contact/institutional or premium | Official market data products | Cleanest legal/commercial posture | Not cheap, not simple retail bar API |
| NYSE/CTA/UTP/SIP feeds | Licensed/vendor | Official consolidated or exchange feeds | Correct for professional display/execution | Too heavy/expensive/contractual for default TyphooN users |

## Are any options 100% free?

Yes, but not with all desired properties.

100% free and useful:

- Yahoo Chart: free/no-key, useful intraday/daily bars, but unofficial and legally
  fragile for a packaged app.
- Stooq: free/no-key daily history, but daily-only and availability/licensing
  needs caution.
- IEX HIST: free official historical IEX exchange data, but T+1 feed files, not a
  realtime chart API.
- SEC EDGAR / NasdaqTrader: free official metadata/fundamentals/reference files,
  not OHLC bars.
- Broker free tiers: Alpaca/IBKR/Tradier can be free to users who have accounts,
  but they are not anonymous and carry entitlement limits.

100% free and **not** realistically available:

- realtime consolidated SIP/NBBO quote feed;
- broad US-equity realtime OHLC bars with redistribution rights;
- full L2/depth across US venues;
- unlimited broad-universe intraday history at 1m granularity.

For a low-resource user base, the honest baseline is:

1. ship Yahoo Chart + Stooq as optional best-effort fallbacks;
2. support user-supplied free broker/API keys;
3. add one or two cheap paid lanes for users who want reliability;
4. never require a paid provider for core charting.

## Recommended implementation priority

### Phase 1 — Hardening the free baseline

1. Keep Yahoo Chart enabled as an optional fallback lane:
   - `yahoo-chart:SYMBOL:TF` namespace;
   - explicit setting/disclaimer;
   - per-symbol no-data tombstones;
   - 429/403 circuit breaker;
   - conservative global rate limit;
   - latest-timestamp freshness scoring.
2. Keep Stooq daily-only:
   - `stooq:SYMBOL:1Day` namespace;
   - provider availability check;
   - pause lane on network/provider failure.
3. Add merge metadata:
   - source per span;
   - percent of visible window from fallback;
   - latest source timestamp;
   - stale/fresh badge.
4. Paint live quote overlays separately from bars:
   - bid/ask lines;
   - spread badge;
   - last quote age;
   - source badge (`Kraken WS`, `Alpaca IEX`, `Yahoo`, etc.).

### Phase 2 — User-key cheap lanes

1. Promote Alpaca free/IEX as a first-class quote/bar assist lane.
2. Add Tiingo as a cheap IEX/reference lane if API terms fit personal/internal
   use.
3. Add MarketData.app as the cheapest paid historical-candle lane to evaluate.
4. Add Twelve Data only after verifying true historical depth and quota behavior
   on real Kraken-equity candidate symbols.
5. Leave Alpha Vantage as deep-history specialist, not broad scheduler default,
   because 25/day free is too restrictive.

### Phase 3 — Best paid bar lane

Evaluate Polygon/Massive as the cleanest low-cost paid historical aggregate lane:

- $29/mo tier can replace a lot of slow broker historical sync if delayed bars are
  acceptable.
- $79/mo tier improves depth.
- $199/mo tier is where realtime starts becoming plausible, but this is no longer
  low-resource friendly.

If implemented, Polygon should be optional and source-provenanced, not a hard
dependency.

## Provider-lane architecture

Add a provider capability registry:

```text
ProviderCapability {
  provider: YahooChart | Stooq | Alpaca | Tiingo | MarketDataApp | Polygon | ...
  asset_classes: Equity | ETF | Crypto | FX | ...
  intervals: [1Min, 5Min, 15Min, 30Min, 1Hour, 4Hour, 1Day, ...]
  max_history_by_interval: duration/unknown
  freshness_class: realtime | delayed_15m | eod | t_plus_1 | unknown
  quote_support: none | last | bid_ask | nbbo | exchange_only
  l2_support: none | exchange_depth | consolidated_depth
  auth: none | api_key | broker_oauth | broker_session
  legal_class: official | broker_entitled | unofficial_best_effort
  request_budget: per_minute/per_day/concurrency
}
```

Scheduler rule:

1. Native broker visible charts and MTF Grid first.
2. Live quote/depth refresh for visible/order-entry symbols next.
3. Fallback front-fill for visible stale bars.
4. Fallback deep-history prepend for visible symbols.
5. Demand-set fallback backlog.
6. Broad catalog fallback only when explicitly enabled and budget-safe.

Do not let a cheap provider create a new unbounded universe-sync cliff.

## Chart and order-entry behavior

Historical candle source and live quote/depth source are independent:

- Candle panel can show `Data: Kraken Equities + Yahoo gap-fill`.
- Bid/ask lines can show `Quote: Alpaca IEX WS age 320ms`.
- L2 panel can show `Depth: Kraken WS` or `Depth: unavailable`.
- Order ticket should show:
  - bid;
  - ask;
  - spread;
  - quote age;
  - quote source;
  - estimated slippage if L2/depth is available;
  - warning if quote source is delayed or stale.

For market orders, stale/delayed quote warnings matter more than candle freshness.
A chart can be visually useful from Yahoo/Polygon/Alpaca bars while the order
button still needs broker-native/current quote state.

## Open questions / verification tasks

1. Verify Yahoo Chart intraday freshness during live market hours for a sample of
   Kraken equities (`AAPL`, `TNDM`, `WOK`, thin/SPAC/unit symbols).
2. Verify MarketData.app actual candlestick freshness and whether its cheap plans
   are delayed enough to make it deep-history-only.
3. Verify Tiingo IEX historical intraday depth and license terms for a desktop app
   with user-supplied keys.
4. Verify Twelve Data historical depth per interval using its earliest-timestamp
   endpoint before integrating it.
5. Decide whether unofficial zero-key sources default on, default off, or default
   on only after a disclaimer acknowledgement.
6. Decide if Polygon/Massive should be the recommended paid lane despite not being
   low-resource friendly for everyone.

## References checked

- Yahoo Chart endpoint: `https://query1.finance.yahoo.com/v8/finance/chart/AAPL?range=1d&interval=1m`
- Yahoo Developer API Terms: `https://legal.yahoo.com/us/en/yahoo/terms/product-atos/apiforydn/index.html`
- Yahoo Terms: `https://legal.yahoo.com/us/en/yahoo/terms/otos/index.html`
- Stooq endpoint pattern: `https://stooq.com/q/d/l/?s=aapl.us&i=d`
- pandas-datareader Stooq docs: `https://pandas-datareader.readthedocs.io/en/latest/readers/stooq.html`
- IEX market data/connectivity: `https://www.iex.io/products/equities/market-data-connectivity`
- IEX legacy market data/HIST notes: `https://iextrading.com/trading/market-data/`
- SEC EDGAR APIs: `https://www.sec.gov/search-filings/edgar-application-programming-interfaces`
- Alpaca data pricing/docs: `https://alpaca.markets/data`, `https://docs.alpaca.markets/docs/about-market-data-api`
- Polygon/Massive pricing/docs: `https://massive.com/pricing`, `https://massive.com/docs/rest/stocks/aggregates/custom-bars`
- Tiingo pricing/docs: `https://www.tiingo.com/pricing`, `https://www.tiingo.com/documentation/iex`
- Twelve Data pricing/docs: `https://twelvedata.com/pricing`, `https://twelvedata.com/docs#time-series`
- Alpha Vantage pricing/docs: `https://www.alphavantage.co/premium/`, `https://www.alphavantage.co/documentation/#intraday`
- Finnhub pricing/candles: `https://finnhub.io/pricing`, `https://finnhub.io/docs/api/stock-candles`
- MarketData.app pricing: `https://www.marketdata.app/pricing/`
- Tradier pricing/docs: `https://tradier.com/individuals/pricing`, `https://docs.tradier.com/reference/brokerage-api-markets-get-timesales`
- Intrinio pricing: `https://intrinio.com/pricing`
- Nasdaq Data Link search/product pages: `https://data.nasdaq.com/search?query=US%20intraday%20stocks`, `https://data.nasdaq.com/databases/NLSP`
- Financial Modeling Prep pricing/docs: `https://site.financialmodelingprep.com/developer/docs/pricing`, `https://site.financialmodelingprep.com/developer/docs#historical-chart`
