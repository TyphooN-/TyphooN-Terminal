# ADR-117: StockTwits Social-Sentiment Ingest into the Research Packet

**Status:** Implemented for the public StockTwits symbol stream. The current terminal also
has a separate Finnhub Reddit/Twitter social-sentiment lane (`BrokerCmd::FetchSocialSentiment`,
`SocialSentimentRow`, and the SENTIMENT research window). Optional historical `sentiment-v2`
and additional social sources remain future work.
**Date:** 2026-06-12
**Related:** ADR-078 (multi-source news ingest pipeline), ADR-079 (research packet — snapshot→SQLite→packet pattern), ADR-080 (web-research ingestion / Return Path), ADR-096 (AI return-path auto-ingest), ADR-008 (centralized rate limiter), ADR-098 (per-frame O(1) discipline), ADR-116 (finviz parity target)

## Context

The research packet (ADR-079) carries fundamentals, ~375 research/indicator surfaces, SEC filings, insider activity, and multi-source **news** (GDELT, Finnhub, Marketaux, Yahoo RSS, NewsAPI, Polygon — ADR-078). Separately, the native research windows can fetch Finnhub Reddit/Twitter social-sentiment rows. As of 2026-06-25, StockTwits public-stream sentiment is also available as a cache-first local research-packet section.

Finviz itself does **not** show social sentiment, so this is an *exceeds-parity* addition (it complements ADR-116, it is not required by it). StockTwits is the most accessible retail-sentiment source and is the natural first social lane.

## StockTwits API (verified 2026-06)

- **Public symbol stream** — `GET https://api.stocktwits.com/api/2/streams/symbol/{SYMBOL}.json`. No auth, free. Returns the ~30 most recent messages for a symbol. Each message may carry a user-applied sentiment tag at `entities.sentiment.basic` ∈ {`Bullish`, `Bearish`} (absent = neutral/untagged), plus `id`, `created_at`, `body`, `user`, and like/reshare counts.
- **Sentiment v2 (aggregated)** — `https://api-gw-prd.stocktwits.com/api-middleware/external/sentiment/v2/{symbol}/detail` (also surfaced via the firestream portal). Returns aggregated bullish/bearish scores, message volume, and participation across timeframes. May require partner/developer access.
- **Rate limits** — unauthenticated access is IP-rate-limited (historically ~200 requests/hour); respond to HTTP 429 with backoff. Authenticated/partner tiers are higher.
- Sources:
  - Public stream pattern: `https://api.stocktwits.com/api/2/streams/symbol/{SYMBOL}.json`
  - Sentiment v2 docs: `https://firestream-portal.stocktwits.com/documentation/sentiment-detail`
  - Sentiment v2 swagger: `https://sentiment-v2-api.stocktwits.com/`

## Decision

Add a **StockTwits ingest lane** as a new research data source feeding the research packet, mirroring the existing news-ingest pattern (ADR-078) and the standard snapshot pipeline (ADR-079: snapshot struct → SQLite table → `BrokerCmd`/`BrokerMsg` → packet emitter). This now exists for the public symbol stream.

1. **Engine fetcher** in `typhoon-engine/src/core/research/providers.rs`: pull the public symbol stream, parse messages, and reduce to a snapshot:
   - `StockTwitsSentimentSnapshot { bullish: u32, bearish: u32, neutral: u32, message_count: u32, bull_bear_ratio: f64, velocity_24h: u32, top_messages: Vec<StockTwitsMessage> }` (type in `research/transcripts_sentiment.rs`).
   - `velocity_24h` = messages in the trailing 24h (momentum of chatter), computed from `created_at`.
2. **Storage** via `research_stocktwits_sentiment` and `upsert_stocktwits_sentiment` / `get_stocktwits_sentiment`; local-cache only, keyed by uppercased symbol.
3. **Packet section** in the research-packet layer (`typhoon-research-ui::packet`, with the native packet dispatcher gathering app/cache context): `### Social Sentiment — StockTwits ({SYM}, as of {ts})` listing bull/bear/neutral counts, bull:bear ratio, 24h velocity, and a few representative recent messages **with provenance + timestamps** (never presented as the terminal's own view).
4. **Scheduling/trigger** through `BrokerCmd::FetchStockTwitsSentiment` and the `STOCKTWITS` / `STWITS` command; **opt-in**, **cache-first**, off the render thread (ADR-098 — no per-frame fetch/parse).
5. **Provider isolation**: the HTTP/JSON is isolated behind `fetch_stocktwits_sentiment` / `parse_stocktwits_symbol_stream`, so swapping the public stream for the sentiment-v2/partner endpoint later is localized.

## Integration points

- `typhoon-engine/src/core/research/transcripts_sentiment.rs` — `StockTwitsMessage` + `StockTwitsSentimentSnapshot`.
- `typhoon-engine/src/core/research/providers.rs` — `fetch_stocktwits_sentiment` + `parse_stocktwits_symbol_stream`.
- `typhoon-engine/src/core/research/storage_core.rs` — schema/upsert/get for `research_stocktwits_sentiment`.
- `typhoon-broker-runtime/src/research_fetch.rs` — `BrokerCmd::FetchStockTwitsSentiment` handler.
- `typhoon-research-ui::packet::stocktwits_sentiment` plus the native packet dispatcher — packet section.

## Risks / constraints

- **Terms of use:** the public endpoint is for personal, non-redistribution use. Keep ingest **local-cache only, user-triggered, no rebroadcast** (consistent with how news is cached). Document as opt-in and do not bundle StockTwits content into any synced/exported artifact beyond the local packet.
- **Signal quality:** user-tagged sentiment is noisy and gameable. Present **raw bull/bear counts + velocity with provenance**, not a single derived "buy/sell signal".
- **Endpoint stability:** StockTwits has changed APIs repeatedly; the provider trait + a single namespace contains breakage.
- **Coverage:** crypto/xStock tickers may not map 1:1 to StockTwits symbols; treat empty/404 as a normal no-data tombstone (same convention as Yahoo/news).

## Future TODOs

- [x] Engine fetcher + `StockTwitsSentimentSnapshot` type + storage helper.
- [x] Research-packet `### Social Sentiment — StockTwits` section.
- [x] **Social Sentiment surface with history (2026-07-03):** the `SENTIMENT`
  window gained keyless "Fetch StockTwits" / "Fetch Reddit" lanes, latest
  snapshot summaries, top-post/message provenance, and a bull/bear + mention
  sparkline drawn from `research_social_history` — every snapshot upsert
  appends a history point (bounded to 500 per symbol × source, ADR-121-style
  retention), so the series accumulates locally with zero extra API calls.
- [x] ~~**sentiment-v2** endpoint~~ **Superseded (2026-07-03):** the purpose
  of the gated historical endpoint was a sentiment/volume time series; the
  local `research_social_history` accumulation now provides that durable
  series from the public stream alone, so the undocumented/gated v2 endpoint
  is deliberately not integrated.
- [x] **Reddit lane (2026-07-03):** keyless `fetch_reddit_mentions` searches
  r/wallstreetbets + r/stocks + r/investing + r/StockMarket (public
  `search.json`, trailing day, descriptive UA) into `RedditMentionSnapshot`
  (`research_reddit_mentions` + history). Per the signal-quality rule above
  it presents **raw mention counts + engagement + post provenance, never a
  derived buy/sell signal** (Reddit has no bull/bear tags). Packet section
  `### Social Sentiment — Reddit mentions`; `FetchRedditMentions` command;
  same local-cache-only / user-triggered / no-rebroadcast posture. This
  closes the Reddit gap noted in ADR-092.

## Consequences

- Adds a **retail social-sentiment surface that finviz does not offer**, advancing the "exceed finviz" half of ADR-116.
- Introduces one new optional external dependency; gated/opt-in, cache-first, rate-limited, render-thread-free.
- Establishes the social-ingest seam (provider trait + namespace + snapshot) that a future Reddit/X lane reuses.
