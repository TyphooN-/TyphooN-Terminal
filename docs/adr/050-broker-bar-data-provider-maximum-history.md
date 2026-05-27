# ADR-050: Broker Bar Data — Provider-Maximum History

**Status:** Implemented | **Date:** 2026-04-05 | **Updated:** 2026-05-20

## Context

TyphooN needs maximum practical bar depth for every enabled broker symbol/timeframe, not arbitrary local windows such as 10,000 / 50,000 bars or "shallow cache" thresholds. The correct depth is provider-specific:

- If the broker API exposes paginated historical bars, TyphooN syncs until the provider is exhausted.
- If the broker API exposes only a bounded recent OHLC window, TyphooN syncs that whole provider window and keeps it current.
- If a provider reports that a snapshot was snipped/truncated, TyphooN keeps paging instead of treating the partial snapshot as complete.

## Decision

Automated broker sync uses provider-maximum semantics:

1. Full-history providers use an effectively unbounded target (`u32::MAX`) for scheduling and persist a backfill-complete marker only after the provider has been exhausted.
2. Limited-window providers use documented/provider-observed window sizes as convergence targets.
3. Backfill-complete markers suppress only repeat historical backfill. Missing/stale/incremental freshness still runs forever.
4. MT5 remains an ingested source: maximum depth is whatever BarCacheWriter / the terminal has exported into SQLite.

## Provider behavior

### Alpaca

- Uses `get_all_bars(...)` for first sync and incomplete-cache backfill.
- Paginates with Alpaca `page_token` until `FetchOutcome::Complete`.
- No local 10k/50k/30k/3.5k/240 bar target caps remain in automated sync.
- When the provider is exhausted, TyphooN stores the actual returned count as the backfill-complete depth.

### tastytrade / DXLink

- tastytrade has no REST OHLCV history endpoint; DXLink Candle snapshots are the source of truth.
- First sync starts at the configured historical floor (`2000-01-01`).
- DXLink `SNAPSHOT_SNIP` is treated as "more history exists", not success. TyphooN advances from the last candle and requests more pages until `SNAPSHOT_END`, timeout/error, or the safety page guard is reached.
- Backfill-complete is persisted on `SNAPSHOT_END` or safety-guard saturation, using the actual stored count. Guard saturation is logged explicitly so the operator can tell "provider keeps snipping" apart from clean exhaustion.

### Kraken Futures

- Kraken Futures chart candles support explicit `from`/`to` ranges.
- First sync starts at the Kraken Futures historical floor (`2018-01-01`) and chunks forward to now.
- The request guard is sized for full 1-minute history from that floor, not a shallow recent window.
- Backfill-complete uses the actual returned count.

### Kraken Spot / xStocks

- Kraken public Spot OHLC is a bounded recent-window endpoint. TyphooN cannot manufacture deep history from this API alone.
- Spot sync therefore requests the full recent provider window (roughly 720 OHLC rows per supported interval; monthly is aggregated from daily) and keeps it current.
- Deep crypto history remains CryptoCompare's role in the source hierarchy.

## Consequences

- No arbitrary 10,000 / 50,000 / 7,500 / 3,500 target-depth caps control broker sync where the API supports full history.
- Full-history first syncs can be expensive. Queue windows, permits, provider rate limits, and backfill-complete markers are mandatory to prevent repeated waste.
- Kraken Spot remains intentionally different because the API is different: it is recent-window sync, not full-history sync.
