# ADR-116: TA-Lib + Godel Parity Round 9 — SEAG / COR / TRA / TECH / SKEW

**Status:** Accepted
**Date:** 2026-04-14
**Supersedes/extends:** ADR-108, ADR-109, ADR-110, ADR-111, ADR-112, ADR-113, ADR-114, ADR-115
**Related:** `engine/src/core/research.rs`, `native/src/app.rs`, `engine/src/core/lan_sync.rs`

## Parity classification

| Feature | Godel documented? | TA-Lib primitive? | Research packet | egui popup | Chart overlay |
| --- | --- | --- | --- | --- | --- |
| SEAG (seasonality) | Yes | No | Yes | Yes | No (deferred — ADR-188) |
| COR (correlation matrix vs peers) | Yes | No | Yes | Yes | No (deferred — ADR-188) |
| TRA (total return analysis) | Yes | No | Yes | Yes | No (deferred — ADR-188) |
| TECH (technical indicators snapshot) | Canonical (all terminals) | No (compositional) | Yes | Yes | No (deferred — ADR-188) |
| SKEW (volatility skew) | Yes | No | Yes | Yes | No (deferred — ADR-188) |

**Round classification:** mostly Godel-Terminal-documented research surfaces (seasonality, correlation, total return, vol skew); TECH bundles several canonical indicators into one packet-oriented view but is not a new indicator itself.

## Context

Round 8 (ADR-115) closed the "price-derived risk analytics + fair value anchor"
gap with HRA / DCF / SVM / OMON / IVOL. With options now cached, with fair value
triangulated from multiple anchors, and with historical return/risk surfaced,
the next visible gap versus Godel Terminal was **time-series pattern analytics**
— seasonality, cross-asset correlation structure, total-return attribution,
classical technical indicators, and the volatility surface across strikes.

Round 9 picks up the five surfaces that fit TyphooN's research-packet pattern
and eliminate that gap. Crucially, **all five are pure compute over existing
caches** (`HP` bars, `DVD` dividends, `PEERS` sector peers, `OMON` chain):

1. **SEAG — Seasonality.** Godel's seasonal-patterns tab shows monthly return
   averages plus day-of-week bias. TyphooN already caches 5Y+ daily bars via
   the `HP` window (ADR-112); SEAG buckets them by calendar month and by
   weekday and reports the distribution.
2. **COR — Correlation Matrix.** Godel's correlation view computes Pearson
   correlation of a symbol against its sector peers over a user-selected
   window. TyphooN already caches peers (ADR-109 PEERS) and daily bars per
   peer (ADR-112 HP); COR is a pure log-return / date-intersect / Pearson
   compute.
3. **TRA — Total Return Analysis.** Godel's "total return" view reports
   price-only return plus dividend yield contribution across standard windows
   (1M / 3M / 6M / YTD / 1Y / 3Y / 5Y / ITD). TyphooN already caches daily
   bars (ADR-112 HP) and dividend history (ADR-109 DVD); TRA sums them per
   window.
4. **TECH — Technical Indicators.** Godel's indicator tab shows a compact
   snapshot of the standard set (RSI / MACD / BB / ATR / ADX / Stoch) with
   a bullish / bearish consensus. TyphooN has always had these on the chart,
   but the research packet had no structured technical view — TECH exposes
   them as cached per-symbol rows the model can reason over.
5. **SKEW — Volatility Skew.** Godel's skew view plots call vs put IV by
   strike for each expiry. TyphooN already caches the chain via OMON
   (ADR-115); SKEW merges calls+puts by strike, picks ATM, and reports the
   ±10% OTM put-vs-call IV gap as a skew proxy.

The standing directive applies: *"continue combing over vs godel parity until
we cannot add more. rinse/repeat do not worry about round count."*

## Decision

Add five new research surfaces following the Round 7/8 pattern verbatim:

### Engine (`engine/src/core/research.rs`)

- **New structs** (ADR-116 section near line 829):
  - `SeasonalityMonth` + `SeasonalityDow` + `SeasonalitySnapshot` — one
    monthly row (avg / median / stdev / positive-year count / min / max),
    one day-of-week row (avg / positive-day count / total), and the
    per-symbol wrapper with years covered, best/worst month, and note.
  - `CorrelationCell` + `CorrelationMatrix` — one peer row (ρ, N, β) and
    the per-symbol matrix (cells, window days, mean ρ, highest/lowest
    peer, note).
  - `TotalReturnWindow` + `TotalReturnSnapshot` — one window row (label,
    trading days, price return, dividend yield, total return, annualized,
    dividends paid, n dividends) and the per-symbol wrapper (last close,
    trailing-12m dividends, trailing-12m yield, windows, note).
  - `TechnicalIndicator` + `TechnicalSnapshot` — one indicator row (name,
    value, optional secondary / tertiary, signal string, note) and the
    per-symbol wrapper (last close, indicators, trend summary, note).
  - `SkewPoint` + `SkewExpiry` + `VolatilitySkew` — one strike point
    (strike, moneyness %, call IV, put IV, combined IV), one expiry
    bucket (expiration, DTE, ATM IV, points, 25Δ P/C skew, term note),
    and the per-symbol wrapper (underlying price, expiries, note).
- **New compute fns:**
  - `compute_seasonality_snapshot(symbol, as_of, bars_oldest_first)` —
    pure compute. Buckets bars by `(year, month)` into a `BTreeMap` to
    capture first/last close per month, builds monthly returns, then
    aggregates month → distribution. Uses Zeller's congruence on the
    date string to derive day-of-week without a chrono dependency.
    Picks best/worst month by mean return.
  - `compute_correlation_matrix(symbol, as_of, window_days, subject_bars,
    peer_series)` — pure compute. Truncates to `window_days` most-recent
    bars, builds log-returns, intersects dates with each peer via a
    `HashMap<date, return>`, then computes Pearson ρ and OLS slope β per
    peer. Skips peers with fewer than 30 common observations. Sorts the
    resulting cells by |ρ| descending.
  - `compute_total_return_snapshot(symbol, as_of, bars, dividends)` —
    pure compute. For each standard window, picks the start bar by
    trading-day offset, sums dividends that fall within `[start_date,
    as_of]`, computes price return and dividend-yield contribution, and
    reports both plus the sum (total return). Annualizes using
    `total_return^(365/window_days) - 1`. Trailing-12m uses a naive
    "year-minus-one" cutoff on `as_of`.
  - `compute_technical_indicators(symbol, as_of, bars)` — pure compute.
    Builds RSI(14) with Wilder smoothing, MACD(12,26,9) with EMA + signal
    + histogram, Bollinger Bands(20,2) + %B, ATR(14) with Wilder
    smoothing, ADX(14) with +DI/−DI, and Stochastic(14,3) %K/%D. Signal
    strings follow the classical levels (`RSI > 70` → OVERBOUGHT,
    `MACD_hist > 0` → BULLISH, etc.). Trend summary counts bullish vs
    bearish signals and emits `BULLISH` / `BEARISH` / `NEUTRAL`.
  - `compute_volatility_skew(symbol, as_of, chain)` — pure compute.
    Merges calls + puts per expiry by `strike × 100` integer key, picks
    ATM as the strike nearest to the underlying, computes put/call IV
    per merged strike, and reports the ±10% OTM put IV minus ±10% OTM
    call IV as the `put_call_skew_25d_pct` proxy.
- **Private helpers:** `ema`, `wilder_smooth`, `zellers_day_of_week`,
  `pearson_correlation`. All inlined in `research.rs`.
- **Schema v9:** `create_research_tables_v9` adds `research_seasonality`,
  `research_correlation`, `research_total_return`, `research_technicals`,
  and `research_vol_skew` (all per-symbol, JSON-blob column). Each table
  has an `updated_at` index for LAN-sync delta selection.
- **Upsert / get pairs:** five new `upsert_*` / `get_*` functions matching
  the Round 7/8 style (JSON blob column, unconditional upsert on conflict).
- **Tests:** 14 new tests (5 roundtrip + 9 compute):
  - `seasonality_snapshot_roundtrip` — cache write/read symmetry.
  - `correlation_matrix_roundtrip` — cache write/read symmetry.
  - `total_return_snapshot_roundtrip` — cache write/read symmetry.
  - `technicals_snapshot_roundtrip` — cache write/read symmetry.
  - `vol_skew_roundtrip` — cache write/read symmetry.
  - `compute_seasonality_on_monthly_uptrend` — all months positive under
    a monotone drift, best month ≠ worst month.
  - `compute_seasonality_on_empty_returns_note` — empty input produces
    the no-data note and an empty monthly table.
  - `compute_correlation_matrix_perfect_copy` — a peer built from the
    subject with mild noise returns ρ ≈ 1.0. Uses alternating drift
    (`±0.005`) to guarantee non-zero variance in both series.
  - `compute_correlation_matrix_skips_empty_peers` — peers with <30
    common observations are skipped silently.
  - `compute_total_return_with_dividends_sums_windows` — synthetic
    300-bar series + three 2024 dividends; verifies every window's
    total return equals price + dividend yield.
  - `compute_technical_indicators_on_uptrend_is_bullish` — monotone
    uptrend bars produce a BULLISH trend summary.
  - `compute_technical_indicators_insufficient_bars_returns_note` —
    <30 bars returns an empty-indicator snapshot with the note set.
  - `compute_volatility_skew_basic_smile` — synthetic chain produces
    non-empty points and the skew sign matches the injected smile.
  - `compute_volatility_skew_empty_chain_returns_note` — empty chain
    returns the no-data note.

### LAN sync (`engine/src/core/lan_sync.rs`)

Added five entries to `SYNCABLE_TABLES`, five `CREATE TABLE` branches in
`create_table_sql()`, and five `updated_at` mappings in
`table_timestamp_column()`. Schema v9 tables replicate across TyphooN nodes
using the same delta protocol as Round 6/7/8.

### Native app (`native/src/app.rs`)

Following the Round 8 surface-addition protocol verbatim:

- **5 new `BrokerCmd` variants:** `ComputeSeasonalitySnapshot`,
  `ComputeCorrelationMatrix`, `ComputeTotalReturnSnapshot`,
  `ComputeTechnicalsSnapshot`, `ComputeVolSkewSnapshot`. Every compute
  runs on the broker thread; the COR variant carries a `peer_series_json`
  string because SQLite reads for per-peer bar series happen on the main
  thread.
- **5 new `BrokerMsg` variants:** `SeasonalitySnapshotMsg`,
  `CorrelationMatrixMsg`, `TotalReturnSnapshotMsg`, `TechnicalsSnapshotMsg`,
  `VolSkewSnapshotMsg`.
- **5 new `TyphooNApp` state fields** (`show_*`, `*_symbol`,
  `*_snapshot`, `*_loading`) plus `cor_window_days: usize` (default
  `252`, range `30..=1260`).
- **5 new broker handlers** on `tokio::spawn`:
  - `ComputeSeasonalitySnapshot` — reads cached `HP` via
    `get_historical_price`, reverses to oldest-first, calls
    `compute_seasonality_snapshot`.
  - `ComputeCorrelationMatrix` — reads the subject's `HP` on the broker
    thread and deserializes the pre-built `peer_series_json` into
    `Vec<(String, Vec<HistoricalPriceRow>)>`. Peer series are built on
    the main thread (where `SqliteCache::connection()` is safe) using
    `research::get_peers` + `research::get_historical_price`.
  - `ComputeTotalReturnSnapshot` — reads `HP` + `get_dividends` on the
    broker thread, then calls `compute_total_return_snapshot`.
  - `ComputeTechnicalsSnapshot` — reads `HP` on the broker thread, then
    calls `compute_technical_indicators`.
  - `ComputeVolSkewSnapshot` — reads the cached options chain via
    `get_options_chain`; emits a "no cached OMON chain — run OMON first"
    note when the chain is missing.
- **5 new receive arms** pattern-matching each new `BrokerMsg`, guarding
  UI state by symbol match and upserting unconditionally to SQLite so LAN
  replication catches every compute.
- **5 new egui windows** (Round 8-style grids / scroll areas), each with
  Symbol / Use Chart / Load Cached / Compute controls. The COR window
  additionally exposes the `cor_window_days` drag value. The SEAG window
  renders two grids (monthly + day-of-week), the TECH window colors each
  row's signal by BULL/BEAR/NEUTRAL, and the SKEW window renders one grid
  per expiry.
- **5 new palette entries:** `SEAG`, `COR`, `TRA`, `TECH`, `SKEW` plus the
  obvious aliases. `CORRELATION` and `INDICATORS` are intentionally
  **omitted** from the COR / TECH alias lists because legacy dashboards
  already own those tokens — the Round 6/7/8 precedent for legacy
  coexistence. The COR entry uses `COR` / `CORRELATION_MATRIX` /
  `CORR_MATRIX` / `PEER_CORR`; TECH uses `TECH` / `TECHNICALS` /
  `TECHNICAL_INDICATORS` / `TA`.

### Research packet (`investigate_symbols`)

- **Per-symbol section:** adds five new sub-blocks after the IVOL snapshot:
  - SEAG summary (header + monthly table + day-of-week table).
  - COR matrix (header + peer table capped at 10 rows).
  - TRA window summary (TTM header + window table).
  - TECH indicator summary (header + indicator table with signal column).
  - SKEW summary (underlying + nearest-expiry summary + strike table).
- **Section counts updated:** "thirty-two sub-blocks" → "thirty-seven".
  Size cap table gained six new rows (SEAG months, SEAG DoW, COR cells,
  TRA windows, TECH indicators, SKEW points). Packet size estimate
  updated to 14-28 KB single / 130-260 KB 10-symbol.

## Alternatives Considered

- **Shipping SEAG / TRA / TECH / SKEW as live provider fetches.** Rejected:
  all four derive from data TyphooN already caches (HP / DVD / OMON), and
  recomputing locally means the user sees their own history cut rather than
  whatever a vendor cached at a different refresh moment. Pure compute
  also avoids new API dependencies.
- **Computing COR against a fixed universe (e.g., all S&P 500).** Rejected
  for Round 9: we already ingest `research::get_peers` for every symbol the
  user has scraped, and sector peers are the most diagnostic correlation
  set. A cross-sector or index-universe COR mode is an additive surface
  that could layer on later without disturbing the schema.
- **Using a proper chrono calendar for SEAG.** Rejected: TyphooN already
  stores bar dates as `YYYY-MM-DD` strings, and Zeller's congruence gives
  correct weekday mapping in ten lines without dragging in a dependency.
- **25Δ Black-Scholes skew for SKEW instead of ±10% OTM proxy.** Held for
  a future round: proper delta requires a full option-pricing path
  (risk-free rate, time to expiry, dividend yield), while the ±10% moneyness
  proxy is visually identical on most equity skews and requires only the
  cached IV values we already have. Upgrading to real delta is a
  compute-only change when we add the inputs.
- **Multi-stage growth / two-stage DCF extension.** Held for a future
  round: this is a Round 8 follow-on, not Round 9 scope.

## Consequences

### Positive

- Five new research surfaces with **zero new API dependencies** — every
  compute reads from caches TyphooN already populates.
- The research packet now includes seasonal patterns, a peer correlation
  matrix, dividend-aware total return, a classical technical indicator
  snapshot, and the IV skew across strikes — closing the gap to the
  kind of multi-lens briefing a Bloomberg / Godel user expects before
  opening a position.
- SEAG / COR / TRA all reflect the user's own investigation history
  (because they read their own HP / DVD / peer caches) — so running the
  same command on two TyphooN nodes with different cache depths produces
  different snapshots, as expected.
- LAN-sync coverage is still 100% — any node that computes a Round 9
  surface replicates it to every peer via the standard delta protocol.
- Schema v9 migration is purely additive: existing `typhoon_cache.db`
  files create the new tables on first Round 9 invocation via
  `CREATE TABLE IF NOT EXISTS`. No data migration required.

### Neutral

- The COR window builds the peer series JSON on the main thread before
  spawning the compute. That's slightly slower than pushing the reads to
  the broker thread but avoids carrying a `&Connection` across `.await`
  (same Send-safety pattern as Round 7 RV).
- SEAG and TECH both need ≥30 daily bars to produce a meaningful snapshot.
  Missing-bar fallbacks emit an explicit note in the snapshot; the packet
  sub-block is silently skipped when `indicators.is_empty()` or
  `months.is_empty()`, matching the behavior of every other
  cache-dependent sub-block.

### Negative

- The SKEW compute depends on OMON having been run first. When the cached
  chain is missing, SKEW emits "no cached OMON chain" and returns an
  empty-expiries snapshot — the packet sub-block is then silently skipped.
  This is a UX note rather than a correctness issue: the SKEW window's
  hint text tells the user to run OMON before re-triggering SKEW.
- The TECH signal strings are fixed plain-English tokens
  (`BULLISH` / `BEARISH` / `NEUTRAL` / `OVERBOUGHT` / `OVERSOLD`). Models
  reading the packet need to treat these as categorical rather than
  numeric, which is consistent with every other qualitative sub-block
  (Analyst consensus, ESG rating, etc.).

## Implementation Notes

- **Alternating-drift synthetic generator for correlation tests.** The
  first version of `compute_correlation_matrix_perfect_copy` used the
  shared `synth_bars(n, start, daily_drift)` helper with a constant drift,
  which meant *every* return in both series was identical — the Pearson
  denominator collapsed to zero (`var_s = var_p = 0`), the compute
  returned `ρ = 0`, and the test asserted `ρ ≈ 1.0`. Fixed by generating
  the subject series inline with alternating drift:
  ```rust
  let drift = if i % 2 == 0 { 0.005 } else { -0.003 };
  ```
  so the log-return series has real variance and the peer copy is
  genuinely a near-perfect clone of a non-constant signal.
- **Date alignment for TRA test.** The `synth_bars(260, ...)` helper
  generates bars from `2024-01-01` through approximately `2024-10-08`
  (260 trading days × one calendar day each). The first version of
  `compute_total_return_with_dividends_sums_windows` used dividend dates
  in 2025 that fell outside the bar range, so `trailing_12m_dividends`
  was zero and the test failed. Fixed by using dividend dates
  `2024-03-15 / 06-15 / 09-15` within the generated range and setting
  `as_of = "2024-10-15"`.
- **Send-safety on COR.** The correlation handler pattern is the first
  broker handler that needs *both* the subject's bars *and* a collection
  of peer bar series. Rather than carrying two `Arc<SqliteCache>` reads
  across the `.await`, the main thread serializes the peer series to
  JSON up front (via `serde_json::to_string`) and hands it to the
  broker as a `peer_series_json: String`. This matches the Round 7 RV
  pattern verbatim.
- **Signal-column contract.** The TECH packet sub-block formats the
  indicator value based on whether `value_secondary` / `value_tertiary`
  are non-zero — MACD's `main/signal/hist` and Bollinger's `upper/mid/
  lower %B` need three slots, while RSI / ATR / ADX only need one.
  Future indicators that need a fourth slot should extend
  `TechnicalIndicator` with an explicit `value_quaternary` rather than
  repurposing the note field.

## Tests

All existing tests still pass. Round 9 adds 14 new tests, bringing the
engine library suite to **656 tests, 0 failures, 3 ignored**.

Key new tests:

- `seasonality_snapshot_roundtrip` — verifies SEAG upsert/get replaces
  on conflict.
- `correlation_matrix_roundtrip` — per-symbol COR cache roundtrip.
- `total_return_snapshot_roundtrip` — per-symbol TRA cache roundtrip.
- `technicals_snapshot_roundtrip` — per-symbol TECH cache roundtrip.
- `vol_skew_roundtrip` — per-symbol SKEW cache roundtrip.
- `compute_correlation_matrix_perfect_copy` — subject built with
  alternating drift so a near-clone peer returns ρ ≈ 1.0.
- `compute_total_return_with_dividends_sums_windows` — 2024 dividends
  within the synth bar range verify the sum across every window.
- `compute_technical_indicators_on_uptrend_is_bullish` — monotone
  uptrend produces `trend_summary == "BULLISH"`.
- `compute_volatility_skew_basic_smile` — injected smile chain produces
  non-empty strike points and the sign of the skew matches.

## Future Work

- **Cross-sector / index-universe COR** — today COR uses `research::get_peers`
  as the peer set. Let the user override via a text field so they can
  build bespoke comparison sets (ETFs, macro assets, factor proxies).
- **Black-Scholes 25Δ SKEW** — replace the ±10% OTM moneyness proxy with
  a proper delta-indexed IV curve once we cache risk-free rate and
  dividend yield per compute. Schema-additive.
- **Multi-indicator confluence score for TECH** — compute a weighted
  score across RSI / MACD / ADX / BB / Stoch and surface it in the
  trend summary line. Purely presentational.
- **SEAG beyond calendar month / day-of-week** — add week-of-month and
  day-of-month buckets for symbols with strong microstructure (e.g.,
  end-of-quarter window-dressing).
- **TRA currency-translated returns** — for foreign-listed symbols,
  optionally translate via the cached WCR snapshot so the packet can
  show USD-normalized total return alongside the native-currency view.
