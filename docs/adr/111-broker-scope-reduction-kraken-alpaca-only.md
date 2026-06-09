# ADR-111: Broker & Data-Source Scope Reduction — Kraken + Alpaca Only; Darwin/MT5/Tastytrade/CryptoCompare Deprecated to Branches

**Status:** Accepted | **Date:** 2026-06-09

## Context

TyphooN-Terminal historically integrated five trading/data surfaces: **MT5
(Darwinex)** via the BarCacheWriter EA bridge plus the **DARWIN** portfolio
analytics suite, **tastytrade** via DXLink, **Alpaca**, **Kraken** (Spot/xStocks/
Futures), and CryptoCompare for crypto history.

Going forward the project trades **Kraken + Alpaca only**. The MT5/Darwinex,
DARWIN-analytics, and tastytrade integrations are **large and monolithic** and
woven through broker dispatch (`BrokerCmd`/`BrokerMsg`/`OrderBroker`), order
routing, the market-data sync scheduler, session persistence, the UI, and — for
MT5 — the shared SQLite cache layer itself (BarCacheWriter). Approximate footprint
at removal time: **Darwin/Darwinex ~2,300 references, MT5/BarCacheWriter ~720,
Tastytrade ~400**. Carrying and compiling that surface area slows iteration on the
codebase we actually run, and dilutes the optimization focus (frame-time,
sync-throughput, cache contention — see ADR-105, the UI-responsiveness work).

**CryptoCompare** (deep crypto history, 2010+) is also deprecated: Kraken
already supplies the crypto market data the active app needs, so the extra
provider is removed to slim the data-source surface and its maintenance/compile
cost. It follows the same branch-snapshot-and-remove path as the brokers.

We want to keep these integrations **recoverable** (they may return) without
**maintaining or compiling** them in the interim.

## Decision

1. **Trade Kraken + Alpaca only.** Remove the MT5/Darwinex (incl. DARWIN
   portfolio + MT5sync), Tastytrade, and **CryptoCompare** integrations from the
   actively-developed `master` codebase.
2. **Preserve full code on long-lived deprecation branches** —
   `deprecated/darwin`, `deprecated/mt5`, `deprecated/tastytrade`,
   `deprecated/cryptocompare` — snapshots at the pre-removal commit. They are
   **not built, tested, or maintained** in the interim; they exist purely as
   restore points. Restore later via `git checkout deprecated/<x> -- <paths>` or
   cherry-pick.
3. **KEEP the `mql5-compiler` crate** (the standalone MQL5/PineScript/MQL4/
   ThinkScript/… → WASM **transpiler**) and the indicator/strategy compiler that
   uses it. It is a **language tool, not the MT5 broker** — `mt5` (MetaTrader
   broker/EA) ≠ `mql5` (the language). Removing the MT5 broker integration must
   not touch the transpiler.
4. **Keep Kraken/Alpaca and all shared infrastructure intact.** The SQLite cache
   (`conn`/`read_conn`, WAL, `bar_cache`/`kv_cache`/`bar_track`, `put_bars`/
   `put_kv`/`merge_bars`/`get_bars`, zstd) is shared with Kraken/Alpaca and stays.
   Only genuinely MT5-specific cache code is removed (BarCacheWriter `demand.txt`,
   MT5 `bid_ask` ingestion, the `mt5:`/`mt5-darwinex` prefixes, the
   `journal_mode=DELETE` BCW-coexistence shims). Anything ambiguous between
   MT5-only and shared is **kept and flagged**, not guessed.

## Status of the removal

- **Tastytrade — DONE.** Functional removal merged to `master`; dead-code cleanup
  (inert sync subsystem, fields, session persistence, settings UI, dead consts/
  fns) completed; `cargo check --workspace` green, warning-free.
- **Darwin/Darwinex + MT5 — IN PROGRESS** on `chore/rip-out-deprecated-brokers`
  (staged, build-green per integration, never touching Kraken/Alpaca or the shared
  cache). MT5 is done last because of the cache untangle.
- **CryptoCompare — TO DO** (same path: `deprecated/cryptocompare` snapshot, then
  remove the `cryptocompare:` data-source tier + backfill from the active code).

## Consequences

- Smaller, faster-compiling codebase focused on Kraken + Alpaca.
- Orphan on-disk cache rows with `tastytrade:`/`mt5:`/`cryptocompare:` prefixes
  are left inert (no migration); they receive no new data and are ignored.
- Existing credentials for removed brokers in the OS keyring are simply unused.
- The DARWIN analytics windows, FTP/GPU spec scanners, DarwinexRadar, and the
  MT5 BarCacheWriter health-check protocol are gone from the running app.

## Regression guards

- Do **not** reintroduce MT5/Darwinex/tastytrade broker code to `master` — if
  they return, it is via a deliberate restore from the `deprecated/*` branches,
  not incremental re-adds.
- Do **not** remove or weaken `mql5-compiler` or the indicator/strategy compiler
  when touching anything MT5-named.
- The shared SQLite cache must remain fully functional for Kraken + Alpaca; never
  remove shared cache code in the name of "MT5 cleanup."

## Relationship to other ADRs

Supersedes / deprecates the broker-specific decisions for the removed surfaces
(their code now lives only on the `deprecated/*` branches):

- **009** Multi-Broker Architecture — narrowed to Kraken + Alpaca.
- **018** Tastytrade Broker Integration — removed.
- **021 / 022** Data-Source Hierarchy (MT5 master, pluggable brokers) — the
  `mt5:`/`mt5-darwinex` master tier is removed; hierarchy is Kraken/Alpaca/
  CryptoCompare.
- **024 / 026 / 035 / 042** DARWIN import/analytics/GPU/overlay — removed.
- **055** DarwinexRadar, **070** Darwinex Zero web scraping, **097** Darwinex Zero
  USA equity universe — removed.
- **081 / 085** MT5 BarCacheWriter health-check + MT5 EA trading-flow — removed.
- **023** Crypto Data-Source Hierarchy (CryptoCompare ⇆ Kraken) — the
  CryptoCompare tier is removed; crypto history comes from Kraken.

Unaffected and explicitly retained: **040** MQL5 Compiler Pipeline and **066 /
067 / 068** transpiler ADRs (language tooling, kept), and ADR-051 (Kraken full
broker), ADR-087 (Alpaca sync autotuning), the Kraken iapi/WS ADRs (094/095/099/
101/102), and ADR-105 (performance plan) which this decision serves.
