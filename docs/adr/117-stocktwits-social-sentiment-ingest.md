# ADR-117: StockTwits Social-Sentiment Ingest into the Research Packet

**Status:** Proposed
**Date:** 2026-06-12
**Related:** ADR-078 (multi-source news ingest pipeline), ADR-079 (research packet — snapshot→SQLite→packet pattern), ADR-080 (web-research ingestion / Return Path), ADR-096 (AI return-path auto-ingest), ADR-008 (centralized rate limiter), ADR-098 (per-frame O(1) discipline), ADR-116 (finviz parity target)

## Context

The research packet (ADR-079) carries fundamentals, ~375 research/indicator surfaces, SEC filings, insider activity, and multi-source **news** (GDELT, Finnhub, Marketaux, Yahoo RSS, NewsAPI, Polygon — ADR-078). It has **no social-sentiment surface today** (`grep` for `stocktwits`/`social_sentiment` is greenfield).

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

Add a **StockTwits ingest lane** as a new research data source feeding the research packet, mirroring the existing news-ingest pattern (ADR-078) and the standard snapshot pipeline (ADR-079: snapshot struct → SQLite table → `BrokerCmd`/`BrokerMsg` → packet emitter).

1. **Engine fetcher** in `typhoon-engine/src/core/research/providers.rs` (or a new `research/social.rs`): pull the public symbol stream, parse messages, and reduce to a snapshot:
   - `StockTwitsSentimentSnapshot { bullish: u32, bearish: u32, neutral: u32, message_count: u32, bull_bear_ratio: f64, velocity_24h: f64, top_messages: Vec<StockTwitsMessage> }` (type in `research/types.rs`).
   - `velocity_24h` = messages in the trailing 24h (momentum of chatter), computed from `created_at`.
2. **Storage** via a small storage helper module (`storage_social_sentiment_snapshots.rs`) under the established `mod` + `pub use ::*` convention; cache namespace `stocktwits:{SYMBOL}`, zstd KV, TTL-bounded (e.g. refresh ≤ 1×/15min/symbol).
3. **Packet section** in `typhoon-native/src/app/symbol_investigation_packet.rs`: `### Social Sentiment — StockTwits ({SYM}, as of {ts})` listing bull/bear/neutral counts, bull:bear ratio, 24h velocity, and a few representative recent messages **with provenance + timestamps** (never presented as the terminal's own view).
4. **Scheduling** through the existing research/news ingest scheduler with the centralized rate limiter (ADR-008); **opt-in**, **cache-first**, off the render thread (ADR-098 — no per-frame fetch/parse).
5. **Provider isolation**: put the HTTP/JSON behind a small provider trait so swapping the public stream for the sentiment-v2/partner endpoint later is a localized change.

## Integration points

- `typhoon-engine/src/core/research/types.rs` — snapshot + message structs.
- `typhoon-engine/src/core/research/providers.rs` (or `social.rs`) — fetch + parse + reduce.
- `typhoon-engine/src/core/research/storage_social_sentiment_snapshots.rs` — schema/upsert/get, re-exported from `research/mod.rs`.
- `typhoon_broker_runtime::news_ingest` / `typhoon_broker_runtime::news` — schedule + store handler.
- `typhoon-native/src/app/symbol_investigation_packet.rs` — packet section.

## Risks / constraints

- **Terms of use:** the public endpoint is for personal, non-redistribution use. Keep ingest **local-cache only, user-triggered, no rebroadcast** (consistent with how news is cached). Document as opt-in and do not bundle StockTwits content into any synced/exported artifact beyond the local packet.
- **Signal quality:** user-tagged sentiment is noisy and gameable. Present **raw bull/bear counts + velocity with provenance**, not a single derived "buy/sell signal".
- **Endpoint stability:** StockTwits has changed APIs repeatedly; the provider trait + a single namespace contains breakage.
- **Coverage:** crypto/xStock tickers may not map 1:1 to StockTwits symbols; treat empty/404 as a normal no-data tombstone (same convention as Yahoo/news).

## Future TODOs

- [ ] Engine fetcher + `StockTwitsSentimentSnapshot` type + storage helper module.
- [ ] Research-packet `### Social Sentiment — StockTwits` section.
- [ ] Optional floating **Social Sentiment** window: bull/bear sparkline over time from stored snapshots.
- [ ] Optional **sentiment-v2** endpoint for a historical sentiment/volume series (needs access check).
- [ ] Extend the same social lane to **Reddit** (e.g. r/wallstreetbets, r/stocks) as a second source — closes the Reddit gap noted in ADR-092.

## Consequences

- Adds a **retail social-sentiment surface that finviz does not offer**, advancing the "exceed finviz" half of ADR-116.
- Introduces one new optional external dependency; gated/opt-in, cache-first, rate-limited, render-thread-free.
- Establishes the social-ingest seam (provider trait + namespace + snapshot) that a future Reddit/X lane reuses.
