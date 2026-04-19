# ADR-115: Godel Parity Round 8 — HRA / DCF / SVM / OMON / IVOL

**Status:** Accepted
**Date:** 2026-04-14
**Supersedes/extends:** ADR-108, ADR-109, ADR-110, ADR-111, ADR-112, ADR-113, ADR-114
**Related:** `engine/src/core/research.rs`, `native/src/app.rs`, `engine/src/core/lan_sync.rs`

## Parity classification

| Feature | Godel documented? | TA-Lib primitive? | Research packet | egui popup | Chart overlay |
| --- | --- | --- | --- | --- | --- |
| HRA (historical return/risk analysis) | Yes | No | Yes | Yes | No (deferred — ADR-188) |
| DCF (discounted cash flow, FCFF) | Yes | No | Yes | Yes | No (deferred — ADR-188) |
| SVM (stock valuation model synthesis) | Yes | No | Yes | Yes | No (deferred — ADR-188) |
| OMON (options chain monitor) | Yes | No | Yes | Yes | No (deferred — ADR-188) |
| IVOL (implied vol rank / percentile) | Yes | No | Yes | Yes | No (deferred — ADR-188) |

**Round classification:** pure Godel-Terminal-documented research surfaces (rolling return/risk, DCF, valuation synthesis, options chain, IV rank); no TA-Lib primitives in this round.

## Context

Round 7 (ADR-114) closed the "light quant briefing" gap by adding WCR / BETA /
DDM / RV / FIGI. With WACC, DDM, and relative valuation in place, the next
visible gap versus Godel Terminal was **price-derived risk analytics** and a
**cash-flow-based fair value anchor** — plus the underlying options data that
feeds most volatility analytics.

Round 8 picks up the five surfaces that fit TyphooN's research-packet pattern
and eliminate that gap:

1. **HRA — Historical Return / Risk Analysis.** Godel's price-analytics tab
   combines rolling window returns (1D … ITD), volatility, Sharpe / Sortino /
   Calmar, and max drawdown into one view. TyphooN already caches daily bars
   via the `HP` window (ADR-112); HRA is a pure compute over those rows.
2. **DCF — Discounted Cash Flow (FCFF).** TyphooN had DDM (dividend-based) but
   no cash-flow-based fair value. DCF closes the gap with a projection-plus-
   Gordon-terminal model keyed off trailing FCFF and the cached WACC.
3. **SVM — Stock Valuation Model synthesis.** Godel's "fair value summary"
   rolls DCF, DDM, peer multiples, and cost of equity into a single
   triangulated view. TyphooN now has every upstream input cached, so SVM is
   a pure compute that turns existing data into a multi-anchor picture.
4. **OMON — Options Chain Monitor.** TyphooN had no options data at all. Yahoo
   Finance's `/v7/finance/options/{SYMBOL}` endpoint is free and returns the
   nearest expiration's calls + puts with IV, volume, and OI — exactly the
   minimum data set to compute IV rank and produce a regime summary.
5. **IVOL — Implied Vol Rank / Percentile.** Godel's IV-rank tile reads the
   current ATM IV against a trailing 52-week history. TyphooN builds the
   history in-place by appending each OMON compute to the previous IVOL
   snapshot's history — no separate historical IV service needed.

The standing directive applies: *"continue combing over vs godel parity until
we cannot add more. rinse/repeat do not worry about round count."*

## Decision

Add five new research surfaces following the Round 6/7 pattern verbatim:

### Engine (`engine/src/core/research.rs`)

- **New structs** (ADR-115 section near line 680):
  - `HraWindow` + `HraSnapshot` — one rolling-return row and the per-symbol
    bundle, including volatility, Sharpe / Sortino / Calmar, and the max-
    drawdown peak/trough pair.
  - `DcfYear` + `DcfSnapshot` — one projection year (revenue / EBIT / NOPAT
    / FCFF / PV) and the full per-symbol snapshot (base inputs, WACC, tax,
    EV, equity value, implied price).
  - `SvmModelRow` + `SvmSnapshot` — one triangulation row (model / implied
    / upside / confidence / source) and the per-symbol bundle with fair
    low / mid / high and upside %.
  - `OptionContract` + `OptionExpiry` + `OptionsChainSnapshot` — the
    per-contract row, per-expiration bucket, and full chain snapshot.
  - `IvolObservation` + `IvolSnapshot` — one ATM IV history point and the
    per-symbol snapshot (current IV, 52w low/high, rank, percentile, history).
- **New fetchers / compute fns:**
  - `compute_hra_snapshot(symbol, as_of, bars_oldest_first, risk_free_pct)`
    — pure compute over oldest-first daily bars. Builds standard rolling
    windows (1D / 5D / 1M / 3M / 6M / YTD by date prefix / 1Y / 3Y / 5Y /
    ITD), walks bars tracking peak and trough for max drawdown, computes
    log-returns → annualized volatility, Sharpe, Sortino, and Calmar.
  - `compute_dcf_snapshot(symbol, as_of, base_revenue, base_fcff, growth,
    terminal_growth, wacc, tax_rate, years, debt, cash, shares)` — pure
    compute. Rejects degenerate inputs (`tg + 0.005 ≥ wacc`, zero base
    values, zero shares). Projects `last_revenue *= 1+g`, `last_fcff *=
    1+g` per year; terminal value = `last_fcff × (1+tg) / (wacc−tg)`.
  - `compute_svm_snapshot(symbol, as_of, current_price, ddm, dcf,
    peer_pe_median, peer_ev_ebitda_median, peer_pbook_median)` — pure
    compute triangulating up to six model rows (WACC cost of equity,
    DDM, DCF, peer P/E × EPS, peer EV/EBITDA × EBITDA − debt + cash /
    shares, peer P/B × BVPS). Skips rows with `implied ≤ 0`.
  - `fetch_yahoo_options_chain(client, symbol)` — GET
    `https://query2.finance.yahoo.com/v7/finance/options/{SYMBOL}`.
    Parses `optionChain.result[0]` — underlying price +
    `options[0].calls/puts` for the nearest expiration.
  - `compute_ivol_snapshot(symbol, as_of, current_atm_iv_pct, history)` —
    pure compute. Derives 52w low/high from history, computes IV rank as
    `(current - low) / (high - low) × 100`, and IV percentile as the
    fraction of history ≤ current.
- **Private helpers** (no new ones beyond Round 7's OLS helpers).
- **Schema v8:** `create_research_tables_v8` adds `research_hra`,
  `research_dcf`, `research_svm`, `research_options_chain`, and
  `research_ivol` (all per-symbol, JSON-blob column). Each table has an
  `updated_at` index for LAN-sync delta selection.
- **Upsert / get pairs:** five new `upsert_*` / `get_*` functions matching
  the Round 6/7 style (JSON blob column, unconditional upsert on conflict).
- **Tests:** 14 new tests (5 roundtrip + 9 compute):
  - `compute_hra_snapshot_synthetic_series` — builds 300 bars with
    monotonically unique dates (day-counter formula inherited from Round 7)
    so the internal date-keyed structures do not collapse duplicates.
  - `compute_dcf_basic_case` and `compute_dcf_rejects_when_terminal_exceeds_wacc`
    — cover the happy path and the caveat path.
  - `compute_svm_triangulation` — verifies fair low/mid/high and row count.
  - `compute_ivol_rank` / `compute_ivol_percentile` — assert both
    calculations against a synthetic history.

### LAN sync (`engine/src/core/lan_sync.rs`)

Added five entries to `SYNCABLE_TABLES`, five `CREATE TABLE` branches in
`create_table_sql()`, and five `updated_at` mappings in
`table_timestamp_column()`. Schema v8 tables replicate across TyphooN nodes
using the same delta protocol as Round 6/7.

### Native app (`native/src/app.rs`)

Following the Round 7 surface-addition protocol verbatim:

- **5 new `BrokerCmd` variants:** `FetchHraSnapshot`, `ComputeDcfSnapshot`,
  `ComputeSvmSnapshot`, `FetchOptionsChain`, `ComputeIvolSnapshot`.
- **5 new `BrokerMsg` variants:** `HraSnapshotMsg`, `DcfSnapshotMsg`,
  `SvmSnapshotMsg`, `OptionsChainMsg`, `IvolSnapshotMsg`.
- **5 new `TyphooNApp` state fields** plus DCF tuning knobs
  (`dcf_growth_pct`, `dcf_terminal_growth_pct`, `dcf_projection_years`).
- **5 new broker handlers** on `tokio::spawn`:
  - `FetchHraSnapshot` — reads cached HP rows via `get_historical_price`,
    reverses to oldest-first, calls `compute_hra_snapshot`.
  - `ComputeDcfSnapshot` — takes pre-computed raw f64 inputs (base revenue,
    base FCFF, etc.) so the handler stays Send-safe and Fundamentals never
    crosses the spawn boundary.
  - `ComputeSvmSnapshot` — takes pre-serialized JSON strings for DDM / DCF
    snapshots and pre-computed peer-median tuples. Deserializes on the
    broker thread and calls `compute_svm_snapshot`.
  - `FetchOptionsChain` — calls `fetch_yahoo_options_chain` with a short
    user-agent so Yahoo's `query2.finance.yahoo.com` accepts the request.
  - `ComputeIvolSnapshot` — takes the pre-read IV history as JSON so the
    handler stays Send-safe.
- **5 new receive arms** pattern-matching each new `BrokerMsg`, guarding
  UI state by symbol match and upserting unconditionally to SQLite so LAN
  replication catches every fetch.
- **5 new egui windows** (Round 7-style grids / scroll areas), each with
  Symbol / Use Chart / Load Cached / Compute|Fetch controls. The DCF window
  also exposes growth %, terminal g %, and projection year drag values.
  The SVM window performs the pre-compute on the main thread (medians of
  peer P/E, EV/EBITDA, P/B using cached `Fundamentals`), then hands the
  pre-computed tuples to the broker. BVPS is derived as
  `(market_cap / price_to_book) / shares_outstanding` when not directly
  available.
- **5 new palette entries:** `HRA`, `DCF`, `SVM`, `OMON`, `IVOL` plus the
  obvious aliases. `OPTIONS` is intentionally **omitted** from the OMON
  palette entry because an older dashboard already owns that token — the
  Round 6/7 precedent for legacy coexistence.

### Research packet (`investigate_symbols`)

- **Per-symbol section:** adds five new sub-blocks after the FIGI snapshot:
  - HRA summary (two header lines + rolling-return table).
  - DCF summary (three lines + bolded implied price).
  - SVM triangulation table (up to six model rows).
  - OMON summary (two lines — regime only; contracts stay in SQLite).
  - IVOL snapshot (single line — rank / percentile).
- **Section counts updated:** "twenty-seven sub-blocks" → "thirty-two".
  Size cap table gained six new rows. Packet size estimate updated to
  11-21 KB single / 98-198 KB 10-symbol.

## Alternatives Considered

- **Shipping HRA as a live Finnhub / Polygon fetcher.** Rejected: we already
  cache daily bars via HP, and recomputing vol / Sharpe from those bars means
  the user sees *their own* history snapshot rather than whatever the vendor
  cut today. Pure compute also avoids a new API dependency.
- **Two-stage DCF (high growth + stable growth phases).** Held for a future
  round: single-stage with Gordon terminal is the textbook baseline and fits
  our "compact anchor" design goal. A two-stage variant is an additive schema
  bump, not a redesign.
- **Using OpenBB / Alpaca options data for OMON instead of Yahoo.** Rejected
  for the initial round: Yahoo is the only source that's free, unauthenticated,
  and returns both calls and puts with IV in a single JSON blob. TastyTrade
  has richer data but only for our own account and costs latency in the
  broker handshake. Alpaca options require a paid tier. Yahoo stays the
  default; a richer-source `OMON_TT` surface is a natural follow-on.
- **Persisting IVOL history separately from the snapshot.** Rejected for
  Round 8: each `IvolSnapshot` already carries its full history array, and
  the upsert-on-compute flow rolls yesterday's history forward each time we
  recompute. A dedicated `research_ivol_history` table is an additive schema
  bump if we ever need to decouple the series from the snapshot metadata.
- **Full OMON contract table in the research packet.** Rejected: even a
  single expiration can easily run 200+ rows, which would blow up the
  packet size and drown the model in greeks. The packet shows one regime
  line per symbol (DTE, P/C ratio, ATM IV); the full chain lives in the UI
  window and in SQLite for LAN replication.

## Consequences

### Positive

- Five new research surfaces; **zero new API key requirements**. Yahoo's
  options endpoint is anonymous, HRA / DCF / SVM / IVOL are pure compute
  over existing caches.
- The research packet now carries a rolling-return ladder, an explicit DCF
  anchor, a multi-model fair-value synthesis, options-chain regime data, and
  an IV rank — closing the gap to the kind of quant briefing a Bloomberg /
  Godel Terminal user expects before a trade.
- HRA / DCF / SVM / IVOL all derive from user-driven cached data, which means
  they reflect the user's actual investigation history rather than whatever
  the vendor's nearest refresh cycle produced.
- LAN-sync coverage remains 100% — any node that computes a Round 8 surface
  replicates it to every peer via the standard delta protocol.

### Neutral

- Schema v8 migration adds five tables. Existing `typhoon_cache.db` files
  create the new tables on first Round 8 invocation via
  `CREATE TABLE IF NOT EXISTS`. No data migration required.
- IVOL bootstraps its own history. The first `IVOL` compute after running
  `OMON` on a fresh cache produces a snapshot with `observation_count == 1`
  and a trivial rank (`0` or `100` depending on rounding). Meaningful
  52-week data requires either repeated runs over time or a one-time
  historical backfill (future work).
- SVM's peer-median cache assumes PEERS has been run for the subject symbol.
  If the peer list is empty, the peer-multiple rows are skipped and SVM
  falls back to just DDM + DCF + cost-of-equity rows. Consistent with every
  other cache-dependent sub-block in ADR-108 through ADR-114.

### Negative

- Yahoo's options endpoint occasionally returns a **429** under heavy use.
  The handler surfaces the error via `BrokerMsg::Error` rather than silently
  emitting an empty snapshot, so the user sees the rate-limit note in the
  log. No retry/backoff today — relies on the fact that research sessions
  rarely fetch more than a handful of chains in a row.
- DCF assumes a forward FCFF margin = current TTM margin. For companies
  mid-investment cycle (heavy capex compression), the implied price will be
  low because capex is depressing FCFF today. This is a documentation note
  rather than a modeling bug — the alternative (projecting margin recovery)
  introduces a free parameter we'd rather not bake into the baseline.
- OMON's packet block is a single line per symbol. Users who want to reason
  about the full chain need to open the window directly. This is by design
  — the alternative (dumping N expirations × M strikes per symbol) would
  blow out the packet size immediately.

## Implementation Notes

- **`?` inside the `ui.horizontal` closure.** The SVM compute block originally
  computed BVPS via `let mc = self_fund.market_cap?; let shares = ...?;` inside
  the `ui.horizontal(|ui| { ... })` closure — but `ui.horizontal` takes a
  closure that returns `()`, so the `?` operator fails to compile. Fixed by
  wrapping the computation in an immediately-invoked `(|| -> Option<f64> { ...
  })()`, which provides a closure that does return `Option<f64>`.
- **Send-safety on the DCF / SVM handlers.** `Fundamentals` does not have all
  the fields DCF / SVM need (e.g. `revenue`, `ebitda`, `eps`, `book_value_per_share`
  are on `QuarterlyFinancial`, not on `Fundamentals`). Rather than make both
  handlers fetch from SQLite (which would require carrying `&Connection` across
  `.await`), the main thread pre-computes the TTM roll-ups + peer medians and
  hands raw `f64` / JSON-serialized tuples to the broker task.
- **`OPTIONS` palette collision.** The token `OPTIONS` was already owned by an
  older dashboard arm farther down the palette match. Round 8's OMON entry
  omits `OPTIONS` from its alias list so the legacy arm still wins — same
  precedent as Round 6 omitting `INDICES` and `MOVERS`.
- **HP cache order inversion.** `research::get_historical_price` returns rows
  newest-first (the on-disk order), but `compute_hra_snapshot` expects
  oldest-first. The HRA handler explicitly reverses the Vec before calling
  compute.
- **Defaults for DCF knobs.** Growth defaults to 8%, terminal growth to 2.5%
  (rough real-GDP proxy), projection years to 5. These are deliberately
  conservative and user-tunable in the window — the goal is a reasonable
  starting point, not a black box.
- **IVOL history rolling.** Each `IVOL` compute reads the previous snapshot's
  history, drops any same-date entry, appends today's observation, and passes
  the whole series to compute. The snapshot stores its own history, so the
  series grows by one entry per unique trading day.

## Tests

All existing tests still pass. Round 8 adds 14 new tests, bringing the engine
library suite to **642 tests, 0 failures, 3 ignored**.

Key new tests:

- `hra_snapshot_roundtrip` — per-symbol HRA cache roundtrip.
- `dcf_snapshot_roundtrip` — per-symbol DCF cache roundtrip.
- `svm_snapshot_roundtrip` — per-symbol SVM cache roundtrip.
- `options_chain_roundtrip` — per-symbol OMON cache roundtrip.
- `ivol_snapshot_roundtrip` — per-symbol IVOL cache roundtrip.
- `compute_hra_snapshot_synthetic_series` — 300 bars, monotonic dates;
  asserts N populated windows and coherent vol / Sharpe / drawdown.
- `compute_dcf_basic_case` — happy path; `implied_price > 0`.
- `compute_dcf_rejects_when_terminal_exceeds_wacc` — asserts caveat note
  and `implied_price == 0.0`.
- `compute_svm_triangulation` — fair_mid within [fair_low, fair_high];
  row count matches configured inputs.
- `compute_ivol_rank` / `compute_ivol_percentile` — synthetic history;
  rank / percentile match manual computation.

## Future Work

- **Two-stage DCF** — add a high-growth phase before steady-state growth for
  symbols whose TTM growth clearly exceeds terminal. Additive schema bump.
- **OMON_TT / OMON_TASTY** — TastyTrade options chain as an alternative data
  source for users with a live broker session. Richer greeks, delayed quote
  behavior, no rate limit.
- **Historical IV backfill** — a one-time import from Yahoo historical options
  snapshots (or a paid data source) to seed IVOL with a real 52w history
  instead of letting it build up over time.
- **HRA sub-period benchmarking** — compare each rolling window's return to
  SPY's return over the same window, surfacing relative strength per period.
- **DCF sensitivity table** — grid of (growth × WACC) implied prices so users
  can eyeball the model's sensitivity without re-running six computes.
