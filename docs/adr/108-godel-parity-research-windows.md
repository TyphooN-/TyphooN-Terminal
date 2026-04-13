# ADR-108 — Godel Parity: Research Windows & Bulk Scrape

**Status:** Implemented
**Date:** 2026-04-13

## Context

ADR-107 delivered multi-source news (replacing the Finnhub-only pane) but left
the rest of the Godel terminal's research surface unaddressed. Godel had
dedicated windows for DES (company overview), PEERS, EARNINGS, IPO calendar,
PRESS releases, SENTIMENT, TRANSCRIPTS, and GLCO (commodities). TyphooN had
`engine/src/core/research.rs` as a template module with data types, fetchers,
and SQLite persistence already in place — plus `BrokerCmd`/`BrokerMsg` variants
and `TyphooNApp` state fields — but no actual UI bindings and no bulk scraper
wired into the MT5/Darwinex universe.

The user's bar for this release is "rival TradingView; TradingView was inferior
to Godel" — i.e., the research windows are mandatory, not optional.

A separate gap: the existing `FundamentalsScrape` handler iterates the entire
MT5/Alpaca/TastyTrade universe against Yahoo for fundamentals, but didn't touch
the research endpoints (Finnhub profile/peers/earnings/press/sentiment and FMP
transcripts). A user running the fundamentals scrape saw fast Yahoo data land
and then had to manually open each DES/PEERS/ERN window per symbol to populate
the research cache — tedious at scale.

## Decision

### 1. Wire the 8 research windows in `native/src/app.rs::update()`

Each window reads from cached state already populated by existing
`BrokerMsg::*` handlers (see ADR-107), adds a top bar with **Symbol / Use
Chart / Load Cached / Fetch** controls, and shows the data in a typed grid or
two-pane reader. All share a resolved `chart_sym_research` helper computed
once per frame to seed the symbol input from the active chart.

| Window       | Layout                                        | Fetches                                         |
|--------------|-----------------------------------------------|--------------------------------------------------|
| DES          | Collapsible: profile, peers, earnings, press  | FetchCompanyProfile + Peers + Earnings + Press   |
| IPO Calendar | Single grid, ±30 day window                   | FetchIpoCalendar                                 |
| ERN          | Historical actuals vs estimates grid          | FetchEarningsHistory                             |
| PEERS        | Clickable chip list → loads symbol in chart   | FetchStockPeers                                  |
| PRESS        | Scrollable group cards with link-out          | FetchPressReleases                               |
| SENTIMENT    | Source / time / counts / score grid           | FetchSocialSentiment                             |
| TRANSCRIPTS  | Two-pane: quarter list → body                 | FetchTranscriptList + lazy FetchTranscriptBody   |
| GLCO         | Collapsible groups by asset class             | FetchCommoditiesQuotes (Yahoo batch)             |

Transcript body loads lazily on row click: first check SQLite for a cached
body, then dispatch `FetchTranscriptBody` only if missing. This mirrors the
SEC filing viewer cache-first pattern.

### 2. Command palette entries

Added to the string-match dispatcher in `update()`:

```
DES | DESCRIPTION        → open + fetch profile/peers/earnings/press
IPO                      → open + fetch ±30d calendar
ERN | EARNINGS_HISTORY   → open + fetch earnings history (distinct from EARNINGS → calendar)
PEERS                    → open + fetch peers
PRESS                    → open + fetch press releases
SENTIMENT | SOCIAL       → open + fetch social sentiment
TRANSCRIPTS | CALLS      → open + fetch transcript list
GLCO | COMMODITIES       → open + fetch Yahoo commodities batch
```

`EARNINGS` remains bound to the existing earnings *calendar* window, so the
new historical earnings window takes `ERN` (Bloomberg ticker convention).

### 3. `TAS` — Time & Sales live tape

New state on `TyphooNApp`:

```rust
show_tas: bool,
tas_symbol: String,
tas_rows: VecDeque<(String, f64, f64, String, String)>, // sym, price, size, side, ts
tas_paused: bool,
```

The existing `BrokerMsg::StreamTick { symbol, price, size, timestamp }` arm
now also pushes into `tas_rows` when the TAS window is open and the symbol
matches the subscription. Side is inferred from the previous-tick comparison
(uptick = buy, downtick = sell, flat = unchanged). Buffer is bounded at 500
prints (ring-buffer semantics via `VecDeque::pop_back`). Pause/Resume/Clear
controls sit in the top bar.

No new `BrokerCmd` is needed — TAS reuses the WebSocket stream that the chart
already establishes when the user loads a symbol.

### 4. `BrokerCmd::ResearchScrape` — bulk research sweep

New cmd variant mirroring the `FundamentalsScrape` shape:

```rust
ResearchScrape {
    use_mt5: bool,
    use_alpaca: bool,
    use_tastytrade: bool,
    finnhub_key: String,
    fmp_key: String,
},
```

Handler follows the same `std::thread::spawn` + current-thread tokio runtime
pattern to sidestep `!Send` connection bounds (see ADR-107). It:

1. Collects the ticker universe before spawning the thread (async broker
   calls while broker is still in scope).
2. Inside the thread, inside the current-thread runtime, calls
   `research::scrape_and_cache_symbol` per ticker. That helper sequences
   profile → peers → earnings → press → sentiment (Finnhub, ~1.1s between
   calls to respect free-tier rate limits) then transcripts (FMP, ~400ms).
3. Reports progress via `BrokerMsg::FundamentalsProgress` every 10 tickers.

New command palette entry: `RESEARCH_SCRAPE | RSCRAPE`.

Runs entirely independent of `FundamentalsScrape` so Yahoo fundamentals and
research endpoints don't share a rate-limit queue.

## Alternatives considered

- **Inline research fetch inside `FundamentalsScrape`.** Rejected — would
  slow the Yahoo loop by 5-6 seconds per ticker even when the user only
  wanted fundamentals, and the rate-limit queues are independent.
- **One big "research scrape" command that also ran fundamentals.**
  Rejected — fundamentals and research have different failure modes and
  different cadence needs; forcing them together limits UX.
- **Dedicated TAS `BrokerCmd::SubscribeTrades`.** Rejected — `StreamTick`
  already arrives from the chart's WebSocket subscription, so reusing it is
  simpler and zero extra traffic. The downside (TAS must match the chart
  symbol) is acceptable and documented in-UI.
- **Render each research window as a separate file.** Rejected — `app.rs`
  already hosts all ~70 egui windows inline; splitting just this group out
  would create an inconsistent pattern.

## Consequences

**Positive:**

- All 8 Godel research surfaces are now implemented against the same typed
  cache layer that LAN-sync already replicates. A standalone client hitting
  a running cache server sees DES/PEERS/ERN/PRESS/SENTIMENT/TRANSCRIPTS/
  IPO/GLCO data without ever making its own API calls.
- `RESEARCH_SCRAPE` command lets the user warm the entire research cache in
  one go after connecting new brokers — same ergonomics as `EVSCRAPE` for
  fundamentals.
- TAS tape reuses existing WebSocket plumbing — zero additional network
  cost, zero new DB schema.
- The bulk scraper's cooperative throttling (1.1s between Finnhub calls)
  stays inside Finnhub's 60-calls/minute free-tier budget while still
  finishing a 500-ticker sweep in ~50 minutes.

**Trade-offs:**

- TAS only works for the current chart's streamed symbol. Cross-symbol
  tape watching would need a real per-symbol subscription. Not implemented
  because the primary use case ("what just printed on what I'm watching")
  is covered.
- The `RESEARCH_SCRAPE` button sends the Finnhub and FMP keys from the
  app's state across the `std::thread` boundary via move. That's fine
  because they're short-lived owned Strings, but note: a user swapping
  keys mid-scrape won't affect the in-flight scrape.
- `FetchSocialSentiment` / `FetchPressReleases` / `FetchEarningsHistory`
  each individually return one `BrokerMsg` per symbol. The DES window
  routes those back into `desc_*` state only when the symbols match —
  otherwise they go to the dedicated windows. This is a dual-write by
  design, not an accident.
- Transcript AI-summary fields (`transcripts_summary`,
  `transcripts_summary_for`) exist on `TyphooNApp` but are not yet wired
  to a summarizer path. Left in place so a future pass can reuse the
  SEC-filing summarizer (ADR-096) without schema churn.

## Related

- ADR-107 — Multi-source news ingest pipeline (pattern template)
- ADR-096 — SEC filing expansion (two-pane reader pattern; summarizer reuse)
- `engine/src/core/research.rs` — Fetchers and SQLite cache helpers
- `engine/src/core/news.rs` — Companion news module
