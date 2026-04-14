# ADR-114: Godel Parity Round 7 — WCR / BETA / DDM / RV / FIGI

**Status:** Accepted
**Date:** 2026-04-14
**Supersedes/extends:** ADR-108, ADR-109, ADR-110, ADR-111, ADR-112, ADR-113
**Related:** `engine/src/core/research.rs`, `native/src/app.rs`, `engine/src/core/lan_sync.rs`

## Context

Round 6 (ADR-113) closed the "global market context" gap by adding WEI / MOV /
INDU plus the derived WACC snapshot. Round 7 continues the systematic comb
against Godel Terminal's surface area and picks up the next five features that
fit TyphooN's research-packet pattern:

1. **WCR — World Currency Rates.** Godel's FX-overview dashboard. TyphooN
   already ingests Yahoo Finance for WEI; `/v7/finance/quote` also answers for
   FX tickers like `EURUSD=X`, so this adds zero new API dependencies.
2. **BETA — Rolling Beta History.** Godel surfaces rolling 1Y / 3Y / 5Y beta
   against SPY. TyphooN has an FMP `beta` field in `Fundamentals`, but it is a
   single point estimate — we wanted the full rolling-window view and the OLS
   diagnostics (α, R², correlation, N) to let the model reason about regime
   stability.
3. **DDM — Gordon Growth Dividend Discount Model.** A compact fair-value
   anchor that complements the WACC cost-of-equity computed in Round 6. With
   WACC and DVD history already cached, DDM is a pure computation with no
   new fetchers.
4. **RV — Relative Valuation Matrix.** Godel's peer-matrix view: Z-score and
   percentile rank for each valuation metric vs same-sector peers. TyphooN
   already has `research_peers` (ADR-109) and the full `Fundamentals` row for
   each peer, so this is another pure-compute surface that turns cached data
   into structured per-metric output.
5. **FIGI — OpenFIGI Instrument Identifier Lookup.** OpenFIGI's `/v3/mapping`
   endpoint is free and anonymous. Adding it gives the research packet the
   authoritative Bloomberg-style identifiers (share-class FIGI, composite
   FIGI, exchange code, security type) that quant / cross-venue workflows
   expect to see alongside the ticker.

The standing directive applies: *"continue combing over vs godel parity until
we cannot add more. rinse/repeat do not worry about round count."*

## Decision

Add five new research surfaces following the Round 5/6 pattern verbatim:

### Engine (`engine/src/core/research.rs`)

- **New structs** (ADR-114 section near line 524):
  - `CurrencyRate` — single FX pair row (Yahoo ticker, display, base/quote,
    region, price, change, change %).
  - `BetaWindow` + `BetaSnapshot` — one rolling-window observation and the
    per-symbol bundle of windows.
  - `DdmSnapshot` — trailing annual dividend, implied growth, required return,
    implied price, method, note.
  - `RvMetricRow` + `RelativeValuation` — one metric row and the full
    per-symbol snapshot (sector, peer count, rows).
  - `FigiIdentifier` + `FigiSnapshot` — one OpenFIGI row and the per-symbol
    wrapper (a ticker can map to multiple share-class FIGIs).
- **New constants:** `FX_MAJORS_UNIVERSE` — 19 hardcoded Yahoo FX tickers
  spanning Majors / Crosses / EM regions.
- **New fetchers / compute fns:**
  - `fetch_currency_rates(client)` — wraps the existing Yahoo batch
    `/v7/finance/quote` path, preserving the universe's declared order via a
    HashMap round-trip.
  - `compute_beta_snapshot(symbol, market_ticker, as_of, sym_bars, mkt_bars)`
    — intersects dates on the two bar series, builds log-returns, runs OLS
    for 1Y (252d), 3Y (756d), 5Y (1260d) windows, records N / α (annualized)
    / R² / correlation per window. Pure; no network or SQL calls.
  - `compute_ddm_snapshot(symbol, as_of, dividends, required_return_pct, return_source)`
    — groups `DividendRecord` by year from `ex_date`, builds a 5-year CAGR
    clamped to `[-20%, +20%]`, then applies `P = D1 / (r - g)` when `r > g`.
    Emits caveats when the model diverges.
  - `compute_relative_valuation(symbol, sector, as_of, metrics)` — Z-score
    and percentile vs peer values for each `RvMetricInput`, skipping rows
    with fewer than three peer values.
  - `fetch_openfigi_identifiers(client, symbol)` — POST
    `https://api.openfigi.com/v3/mapping` with `[{idType: "TICKER",
    idValue: SYM, marketSecDes: "Equity"}]`.
- **Private helpers:** `ols_regression` and `log_returns` (used by
  `compute_beta_snapshot`).
- **Schema v7:** `create_research_tables_v7` adds `research_currency_rates`
  (singleton snapshot keyed on `'latest'`), `research_beta`, `research_ddm`,
  `research_relative_valuation`, and `research_figi` (all per-symbol).
  Each table has an `updated_at` index for LAN-sync delta selection.
- **Upsert / get pairs:** five new `upsert_*` / `get_*` functions matching
  the Round 6 style (JSON blob column for lists of rows, unconditional
  upsert on conflict).
- **Tests:** 12 new tests (roundtrip + compute):
  - `compute_beta_snapshot_synthetic_2x_market` — builds 300 bars with
    monotonically unique dates (using a day-counter that increments a
    `base_day` bounded to `[1, 28]`) so the date-intersection HashMap does
    not collapse duplicates. Confirms β within 0.01 of 2.0 and R² > 0.99.
  - `compute_ddm_basic_growth` and `_diverges_when_growth_exceeds_return` —
    cover the happy path and the `r ≤ g` caveat path.
  - `compute_relative_valuation_z_scores` — verifies metric skip behavior
    (None value, <3 peers) and percentile ranking.

### LAN sync (`engine/src/core/lan_sync.rs`)

Added five entries to `SYNCABLE_TABLES`, five `CREATE TABLE` branches in
`create_table_sql()`, and five `updated_at` mappings in
`table_timestamp_column()`. Schema v7 tables replicate across TyphooN nodes
using the same delta protocol as Round 6.

### Native app (`native/src/app.rs`)

Following the Round 6 surface-addition protocol verbatim:

- **5 new `BrokerCmd` variants:** `FetchCurrencyRates`, `FetchBetaSnapshot`,
  `ComputeDdmSnapshot`, `ComputeRelativeValuation`, `FetchFigiIdentifiers`.
- **5 new `BrokerMsg` variants:** `CurrencyRatesMsg`, `BetaSnapshotMsg`,
  `DdmSnapshotMsg`, `RelativeValuationMsg`, `FigiSnapshotMsg`.
- **5 new `TyphooNApp` state fields:** `show_*`, `*_symbol`, `*_snapshot` /
  `*_rates`, `*_loading` for each surface.
- **5 new broker handlers** on `tokio::spawn`:
  - `FetchCurrencyRates` — calls `research::fetch_currency_rates`, no cache
    lookup needed.
  - `FetchBetaSnapshot` — sequentially fetches 5Y history for the symbol
    and SPY via `fetch_fmp_historical_price`, then calls
    `compute_beta_snapshot`. Inline compute keeps the spawn Send-safe.
  - `ComputeDdmSnapshot` — reads cached dividends via
    `shared_cache_broker.read().ok().and_then(|g| g.clone())`, then calls
    `compute_ddm_snapshot`.
  - `ComputeRelativeValuation` — takes `self_json` + `peers_json` from the
    UI side (built on the main thread where the SQLite connection lives)
    so the broker thread stays Send-safe.
  - `FetchFigiIdentifiers` — calls `fetch_openfigi_identifiers` and wraps
    the result in a `FigiSnapshot`.
- **5 new receive arms** pattern-matching each new `BrokerMsg`, guarding
  UI state by symbol match and upserting unconditionally to SQLite so LAN
  replication catches every fetch.
- **5 new egui windows** (Round 6-style grids / scroll areas), each with
  Symbol / Use Chart / Load Cached / Fetch|Compute|Lookup controls.
- **5 new palette entries:** `WCR`, `BETA`, `DDM`, `RV`, `FIGI` plus the
  obvious aliases (`CURRENCY`, `ROLLING_BETA`, `GORDON_GROWTH`,
  `RELATIVE_VALUATION`, `OPENFIGI`, etc.). No legacy collisions to work
  around this round.

### Research packet (`investigate_symbols`)

- **Global section:** adds WCR as `Global.4`, emitted once above the
  per-symbol blocks. Renders grouped by region (Majors / Crosses / EM) with
  up to 8 pairs per region.
- **Per-symbol section:** adds four new sub-blocks after the WACC snapshot:
  - Rolling beta table (Window / β / α / R² / Corr / N).
  - DDM summary (trailing D0, g, r with sources, implied price or caveat).
  - RV peer-Z-score table (Metric / Value / Peer Median / Z / Percentile).
  - FIGI identifiers (up to 3 per symbol, each with ticker, FIGI, share
    class FIGI, exchange, description).
- **Section counts updated:** "twenty-three sub-blocks" → "twenty-seven".
  Size cap table gained five new rows. Packet size estimate updated to
  10-19 KB single / 92-182 KB 10-symbol.

## Alternatives Considered

- **Extending the existing `Fundamentals` `beta` field instead of building
  BETA.** Rejected: the Fundamentals beta is a single upstream point
  estimate with no rolling window or diagnostic data. We want the full
  (β, α, R², N, corr) tuple per window so the model can reason about beta
  stability, and we want the ability to recompute for any market proxy.
- **Using a premium DDM library / richer multi-stage model.** Rejected:
  Gordon Growth is the textbook baseline and fits TyphooN's "compact anchor"
  design goal. Extending to H-model or two-stage is easy once we have the
  baseline caching and UI in place — adding a third growth phase later is
  an additive schema bump, not a redesign.
- **Pulling relative valuation from a third-party peer screen service.**
  Rejected: we already cache full `Fundamentals` for every ticker the user
  has scraped, and the `research_peers` table already tells us who the
  sector peers are. Z-scores and percentile ranks are a 20-line pure
  compute — no reason to take a network dependency on a screener API.
- **Using OpenFIGI's bulk-mapping API (up to 100 tickers / call) for FIGI.**
  Held for a later round: per-symbol is simpler to fit the broker-handler
  pattern and OpenFIGI's free tier is generous enough that per-symbol calls
  during research sessions are not rate-limited. If we ever batch FIGI in
  the ASKAI pre-flight, we'll revisit.

## Consequences

### Positive

- Five new research surfaces with zero new API key requirements (OpenFIGI is
  anonymous; WCR reuses Yahoo; BETA / DDM / RV are pure compute over existing
  caches).
- The research packet now includes rolling beta, an explicit DDM anchor, a
  peer-Z-score valuation matrix, and authoritative FIGI identifiers —
  closing the gap to the kind of quant-lite research briefing a Bloomberg
  Terminal user expects before a trade.
- BETA / DDM / RV all derive from cached data, which means they reflect the
  user's own investigation history rather than depending on whatever the
  vendor's nearest refresh cycle produced.
- LAN-sync coverage is still 100% — any node that computes a Round 7
  surface replicates it to every peer via the standard delta protocol.

### Neutral

- Schema v7 migration adds five tables. Existing `typhoon_cache.db` files
  create the new tables on first Round 7 invocation via
  `CREATE TABLE IF NOT EXISTS`. No data migration required.
- RV peer fetching still relies on `research::get_peers` — if a symbol's
  peers list is empty (because PEERS hasn't been run), the RV window shows
  a hint and the packet block is silently skipped. This is consistent with
  every other cache-dependent sub-block.

### Negative

- The beta compute path is the first broker handler in the research family
  that sequentially fetches two independent FMP history endpoints before
  computing, so it can take 6-10 seconds end-to-end on a cold cache. The
  window reports "Loading…" during the fetch, so this is a UX note rather
  than a correctness issue.
- DDM implied prices are deliberately conservative about divergence — when
  `r ≤ g`, the snapshot records `implied_price = 0.0` and a caveat rather
  than producing an unbounded or arbitrarily capped value. Models reading
  the packet need to handle the zero-price case, which is documented both
  in the RESEARCH_PACKET.md sub-block spec and in the DDM window hint.

## Implementation Notes

- **Date-intersection HashMap bug.** The first synthetic-data beta test
  produced β ≈ 1.82 because the naive date template
  `format!("2025-{:02}-{:02}", 1 + (i % 12), 1 + (i % 28))` generated
  duplicate `YYYY-MM-DD` strings as `i` wrapped — the HashMap silently
  collapsed duplicates, dropping returns in both series. Fixed by using a
  day-counter that increments a unique base day for every `i`:
  ```rust
  let base_day = 1 + (i % 28);            // 1..28
  let month    = 1 + ((i / 28) % 12);     // 1..12
  let year     = 2024 + (i / (28 * 12));  // monotonically up
  ```
- **Send-safety on broker handlers.** The RV handler builds the peers
  JSON array on the main thread (where `SqliteCache::connection()` is
  safe) and hands it to the broker task via `self_json` / `peers_json`
  strings. This avoids carrying a `&Connection` across `.await`.
- **Round 6 precedent on palette collisions.** Round 6 intentionally
  omitted `"INDICES"` and `"MOVERS"` from its palette entries to keep
  legacy dashboards accessible. Round 7 has no legacy collisions —
  `WCR`, `BETA`, `DDM`, `RV`, and `FIGI` are all fresh tokens.

## Tests

All existing tests still pass. Round 7 adds 12 new tests, bringing the
engine library suite to **628 tests, 0 failures, 3 ignored**.

Key new tests:

- `currency_rates_roundtrip` — verifies WCR upsert/get replaces on conflict.
- `beta_snapshot_roundtrip` — per-symbol BETA cache roundtrip.
- `ddm_snapshot_roundtrip` — per-symbol DDM cache roundtrip.
- `relative_valuation_roundtrip` — per-symbol RV cache roundtrip.
- `figi_snapshot_roundtrip` — per-symbol FIGI cache roundtrip.
- `compute_beta_snapshot_synthetic_2x_market` — synthetic sample where the
  "symbol" is constructed as 2× the market; β within 0.01 of 2.0,
  R² > 0.99, 3 windows populated.
- `compute_ddm_basic_growth` — 10y × 4q dividends with 7% annual growth;
  required r = 12%; asserts `implied_price > 0` and implied growth within
  [4%, 10%].
- `compute_ddm_diverges_when_growth_exceeds_return` — asserts
  `implied_price == 0.0` and non-empty caveat note when `r ≤ g`.
- `compute_relative_valuation_z_scores` — subject P/E of 30 against 7
  peers with tight distribution; percentile within [60, 80].

## Future Work

- **Batched OpenFIGI mapping** — the free tier allows up to 100 tickers per
  POST. ASKAI pre-flights could pre-populate FIGI for the entire symbol
  list in one call instead of per-symbol.
- **Two-stage DDM** — add a high-growth phase before steady-state growth
  for symbols whose CAGR clearly exceeds required return. Schema-additive.
- **Sector-rotation overlay for BETA** — compare each symbol's 1Y β
  against its sector ETF benchmark rather than SPY only. Requires
  mapping sector → ETF ticker (XLK, XLF, etc.).
- **RV with custom peer lists** — today the RV window consumes
  `research::get_peers` output. Let the user override the peer list via
  the window (text field) so they can build bespoke comparison sets.
