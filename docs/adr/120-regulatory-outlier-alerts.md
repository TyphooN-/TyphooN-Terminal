# ADR-120: Regulatory Outlier Alerts Beside Chart Symbols

Status: Accepted
Date: 2026-06-13

## Context

Some symbols carry regulatory status that is not obvious from price, news, fundamentals, or SEC filings. WOK is currently on the Nasdaq Reg SHO Threshold List. That matters enough to be visible at the point of decision: the chart header next to the symbol.

A Reg SHO threshold security is an outlier condition, not normal market metadata. Hiding it in a research window or requiring a manual web lookup is too easy to miss.

## Decision

TyphooN Terminal will maintain a cached symbol-level regulatory alert layer and render active alerts as red badges attached to the chart symbol header.

Initial alert source:

- NasdaqTrader Reg SHO Threshold List
- Public daily text file under `https://www.nasdaqtrader.com/dynamic/symdir/regsho/`
- No API key, no paid API, no account required

Initial UI label:

- `!! Reg SHO !!`

Storage:

- SQLite table `regulatory_alerts`
- keyed by `(symbol, kind, source)`
- stores label, source, as-of date, details, updated timestamp

Refresh behavior:

- background thread refreshes NasdaqTrader Reg SHO periodically
- cached alerts are read into `BgData`
- chart rendering consumes in-memory `regulatory_alerts_by_symbol`
- no per-frame network or database lookup

Symbol normalization:

- chart symbols such as `WOK.EQ` normalize to `WOK`
- Nasdaq-listed symbols are stored uppercase
- a single normalizer (`regulatory_alerts::normalize_regulatory_symbol`: strip `.EQ`, drop `/`, uppercase) is the **only** way callers key into `regulatory_alerts_by_symbol`. The chart header and the watchlist badge both use it — a plain `to_ascii_uppercase()` lookup silently missed any suffixed ticker and is a recurring bug source.

## Surfaces (2026-06-15)

The alert layer is consumed at three points, all reading the same in-memory `regulatory_alerts_by_symbol` map (no per-frame DB/network):

1. **Chart header badge** — the original surface: `!! Reg SHO !!` drawn before the EXT/daily-close chip so a compliance badge is never the element pushed off the right edge.
2. **Watchlist badge** — the ticker renders red with a `!!` drawn on the *top* layer (after the value columns) so the right-aligned Last/Chg text can never overpaint it. A dedicated sortable column was tried and reverted: it does not fit the packed right panel without pushing Vol/buttons off the edge.
3. **`REG_SHO` command + floating window** — `REG_SHO` (registered in the command palette `COMMANDS` registry) opens a floating `egui_extras` table of every threshold symbol. Columns: Symbol · Last · Bid · Ask · Dly Close · Chg% · Actions · Details.

Window data population:

- The window is **cache-based** ("live from cache"). On open it loads cached **daily** bars for every threshold symbol not already in the watchlist, **off the render thread** (`spawn_blocking` + mpsc, mirroring the MTF-grid loader) to avoid the SQLite-read stall when a bulk bar-sync writer holds the single conn mutex. Results merge into `reg_sho_prices` and fill Last / Dly Close / Chg% for the whole list.
- Live **Bid/Ask** come only from watchlisted symbols (the only ones with a live quote subscription); absent values render `—`, never a misleading `0.0000`.
- Per-row **Actions**: `+WL` (add to watchlist — shows `✓WL` when already present, and forces an immediate quote refresh), `D1` / `W1` (open or focus a chart at that timeframe via `SymbolAction::OpenChartTf`).

Note: an earlier `reg_sho: bool` field on `WatchlistRow` (intended to drive a sortable column) was never populated and has been removed — the map is the single source of truth for Reg SHO status.

## Why not require an API?

No API is needed for Reg SHO. NasdaqTrader publishes a daily machine-readable pipe-delimited text file. The app can use the public TXT feed directly and cache it locally.

An API may be needed later for other regulatory/status sources if they lack public downloadable files, but Reg SHO specifically does not require one.

## Consequences

Positive:

- Reg SHO and similar outlier conditions become visible exactly where the user looks before trading.
- Works offline after the latest successful refresh.
- Avoids adding another credential/API dependency.
- Keeps regulatory warning rendering O(1) in the chart path.

Negative / risks:

- NasdaqTrader availability can fail or be delayed; stale cached data may persist until the next successful refresh.
- This is an informational alert, not legal/compliance advice.
- Additional alert sources will need source-specific parsing and stale-data policy.

## Additional source: trading halts / LULD pauses (2026-06-15)

Implemented as the second free source. NasdaqTrader publishes a public, no-key
RSS feed of current US trading halts and LULD volatility pauses
(`rss.aspx?feed=tradehalts`). It feeds the **same** `regulatory_alerts` table
(`kind = 'trade_halt'`, label `!! HALT !!`) so it renders through the existing
chart-header and watchlist badges with no new UI.

Differences from the Reg SHO list:

- **Transient, not daily.** Halts resolve intraday, so the background loop
  re-fetches on a tight cadence (~2 min vs Reg SHO's 30 min) and **fully
  replaces** the cached `trade_halt` rows each time — no smart as-of skip.
- **Resumed halts are dropped.** An entry with a published resumption trade time
  is no longer halted, so it is excluded; only currently-halted symbols carry a
  badge. An empty feed (all resolved) clears the rows.
- The reason code is mapped to a human description in `details` (e.g. `LUDP →
  Volatility trading pause (LULD)`).

Because the `regulatory_alerts` map is now multi-kind, the `REG_SHO` window
filters to `kind = 'reg_sho_threshold'` so halts don't appear mislabeled there,
and the watchlist red-ticker/`!!` flag means "has any regulatory alert."

## Future Extensions

Status of the remaining candidates (free sources only, per project policy):

- **Short Sale Restriction (Reg SHO Rule 201 / SSR)** — free: derivable from
  data already held (a ≥10% drop from the prior daily close arms SSR for the
  rest of that day plus the next trading day). No feed needed; the open work is
  an intraday trigger + next-trading-day expiry state machine feeding a
  `kind = 'ssr'` alert. Next free extension to build.
- **SEC Fails-to-Deliver / FINRA daily short-sale volume** — free, public,
  machine-readable, but bulk + delayed (semi-monthly / T+1); better as an
  enrichment metric than a real-time badge.
- **Exchange delisting / non-compliance notices** — no reliable free
  machine-readable consolidated feed identified; deferred pending a source.
- **Hard-to-borrow / borrow-rate feeds** — **deferred: requires a paid data
  source** (IBKR account / Ortex / S3 etc.). Out of scope per the free-source
  policy.

Each new free source should feed the same `regulatory_alerts` table and render
as compact chart-header / watchlist badges.
