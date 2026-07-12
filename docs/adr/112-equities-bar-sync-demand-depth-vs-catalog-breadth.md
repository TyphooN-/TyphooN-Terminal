# ADR-112: Equities Bar-Sync — Demand-Depth vs Catalog-Breadth Provider Division

**Status:** Accepted / revised for bounded full-universe native WS coverage | **Date:** 2026-06-11

Revises the streaming-scope decision in **ADR-099** (Kraken WS full-universe
streaming) and extends **ADR-102** (Kraken-equities gap-fill via Alpaca) and
**ADR-103** (dedicated provider lanes). Rate mechanics live in **ADR-101**
(iapi AIMD) and **ADR-095** (rate-limit pacing).

## Context

TyphooN is a **research terminal**: research packet, outlier scans, screeners,
and backtests all need broad, deep, correct historical bars across the full
~12k xStock/equity catalog and every enabled timeframe — not just the symbols
being actively traded.

Two regressions had quietly defeated that goal:

1. **iapi was pointed at the whole catalog.** `kraken_equity_native_history_symbols`
   was flipped from demand-scoped to a full-catalog sweep on the assumption that
   "iapi now sustains ~40 req/s." It does not. The grounded, AIMD-discovered
   ceiling is **~6 req/s** (7+ trips Cloudflare 1015 IP bans; see ADR-101 and
   `sync_config.rs`). One (symbol, tf) per call × 12k × 8 TF = ~98k calls = 4.5h
   minimum with zero bans, multi-day with them. Observed overnight result:
   **Kraken Equities 0.7% synced.**
2. **WS OHLC subscribed the full ~12k catalog × 8 intervals** (ADR-099). A single
   WS v2 connection cannot hold that; Kraken reset the connections continuously
   ("connection reset without closing handshake" storms), each reset re-snapshot
   bursting into the SQLite writer and stalling egui for up to 16 s. Large
   universes were also subscribed with `snapshot=false`, so even while connected
   they backfilled **no** history — only live forming bars.

Net: the two mechanisms that were *supposed* to give Kraken-native catalog
coverage delivered ~zero coverage and most of the UI stalls.

A further constraint discovered during this work: **Kraken sources its
stock/ETF/equity bars from Alpaca on the backend.** Kraken-native and Alpaca bars
therefore share a price scale and a failure mode — they are not independent. See
ADR-113.

## Decision

Split equity bar-sync into a **demand-depth lane** and a **catalog-breadth
lane**, and assign each provider the job it is actually good at.

### Provider roles

| Lane | Provider(s) | Job | Scope |
|---|---|---|---|
| Live stream | Kraken WS OHLC | current/forming bar, freshness | **demand only** (held / open chart / watchlist) |
| Native depth-repair | Kraken iapi (~6 req/s) | cold-start / gap repair of Kraken-native bars | **demand only** |
| Native catalog history | Kraken WS OHLC **snapshot sweep** | Kraken-native recent-window history for the catalog | catalog, paced/rotating |
| Catalog breadth | Alpaca (multi-symbol batched) | deep history MN1→M15 across catalog | catalog |
| Assist | Yahoo | recent intraday window + deep daily backup | catalog |
| Merged | — | the chart/research-usable combined series | catalog (see ADR-113) |

### Rules

1. **iapi never sweeps the catalog.** It is a demand-scoped depth-repair lane
   only. A demand set (typically dozens of symbols) clears across all timeframes
   in ~a minute at 6 req/s.
2. **WS live streaming is demand-scoped.** Never hold permanent subscriptions for
   the full catalog. (Coexists with the snapshot sweep below — different
   mechanism.)
3. **Kraken-native catalog history comes from a paced, rotating WS OHLC
   *snapshot sweep*** — subscribe a bounded batch *with* `snapshot=true`, drain
   the snapshot bars (they persist to `kraken-equities:*`), unsubscribe, advance
   the cursor, rotate; high-timeframe-first. WS is bounded by
   subscription-count-per-connection, **not** the Cloudflare 1015 wall that caps
   iapi, so a bounded sweep covers the catalog in ~tens of minutes. Snapshot
   depth is a recent window per interval (deep on D1/W1, recent on intraday —
   which is full history for most short-listed xStocks). Building blocks already
   exist: `build_subscribe_frames_with_snapshot(.., true)` and
   `build_unsubscribe_frame`.
4. **Alpaca is the catalog breadth workhorse** via the multi-symbol
   `/v2/stocks/bars` endpoint (`get_stock_bars_batch_targeted`), grouped by
   timeframe and chunked. Size its queue/batch to saturate the detected RPM tier
   so its high-TF ladder fully descends MN1→W1→D1→H4→H1→M30→M15.
5. **Scheduling is strict high-timeframe-first** across the catalog
   (MN1→W1→D1→…→M1). This is intentional: research wants the high TFs complete
   first. Each provider's ladder descends independently; a slow lane must never
   gate a fast one.
6. **M5/M1 across the catalog are recent-window, not deep.** Deep 1-minute ×
   12k × years is ~10⁸+ bars / tens of GB, and Yahoo only serves ~7 days of
   1-minute. Alpaca already defaults M1/M5 to fresh-only (`alpaca_sync_target_bars`
   returns `None`). Deep intraday is reserved for the demand set.
7. **Sync Status separates native, assist, and merged coverage.** The native
   full-universe denominator covers enabled native timeframes `1Min` through
   `1Week`; native `1Month` is derived from daily data. iapi remains demand-only,
   while bounded WS snapshot waves provide native catalog coverage. Assist and
   Merged rows retain their own provider/full-catalog semantics.

## Regression guards (do-not list)

These are the specific footguns that caused the 2026-06 regressions. Do not:

- **Do not** sweep the catalog through iapi. It is ~6 req/s; assume that ceiling
  permanently unless a *measured* re-discovery proves otherwise. Do not bake an
  optimistic constant ("~40 req/s") into a scheduling decision.
- **Do not** hold permanent WS subscriptions for the full catalog (any count
  above the single-connection limit, ~5k, thrashes — see ADR-099).
- **Do not** subscribe large universes with `snapshot=false` and expect history;
  the snapshot sweep must request snapshots.
- **Do not** deep-backfill M1/M5 across the whole catalog.
- **Do not** let one provider's lane (especially the slow iapi lane) gate
  another's coverage or starve the high-TF ladder.

## Consequences

- Demand symbols (what you trade/watch) retain foreground priority and iapi
  depth repair. Native full-universe WS work advances strict high-timeframe-first
  from `1Week` through `1Min` in bounded waves; `1Month` is derived from daily.
  Alpaca/Yahoo continue to provide deep assist and merged breadth.
- WS reconnect churn and the multi-second SQLite-contention UI stalls are
  removed (the 12k permanent-live subscription is gone).
- Kraken-native catalog coverage is delivered by the snapshot sweep instead of
  the (rate-limited, ineffective) iapi sweep.
- Research functions get broad, high-TF-complete data without a 12k-symbol
  request cliff.

## Status of implementation (2026-06-11)

- **Done:** demand-scoped persistent WS live subscriptions + iapi depth repair;
  full-universe native timeframe coverage is supplied by bounded WS snapshot
  waves and reported separately from assist/merged rows.
- **Done:** Kraken WS OHLC snapshot sweep (rule 3). The app now schedules one
  bounded xStocks catalog batch at a time across enabled native timeframes,
  high-timeframe-first, requests
  `snapshot=true`, drains through the existing bounded writer, unsubscribes, and
  advances a persistent in-session sweep cursor.
- **Done:** Alpaca/Yahoo catalog breadth lanes are right-sized for full-tilt
  high-TF-first convergence via bounded queue/batch/scan windows
  (`sync_config.rs`, `market_data_sync.rs`).
- **Done:** Yahoo `adjclose` ingestion (ADR-113) uses split/dividend-adjusted
  close as the independent corroborator scale.
