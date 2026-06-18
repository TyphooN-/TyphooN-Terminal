# Architecture Decision Records

This directory is intentionally compacted for human onboarding.

Rules:

1. One ADR per durable architectural decision, not per work session.
2. No more numbered parity passes or execution-log ADRs.
3. If work follows an existing architecture, update the existing ADR instead of creating a new one.
4. Historical execution detail lives in git history; the top-level ADR set should stay readable.

Current count: 120 ADRs (removed-feature ADRs were deleted 2026-06; numbers are not reused, so gaps in the sequence are expected). ADR-111 records the broker & data-source scope reduction to **Kraken + Alpaca only** — Darwin/Darwinex, MT5/BarCacheWriter, Tastytrade, and CryptoCompare are deprecated to `deprecated/*` branches (preserved as restore points, not built or maintained in the interim). ADR-112 records the equities bar-sync split into a **demand-depth lane** (Kraken WS live + iapi, demand-scoped) and a **catalog-breadth lane** (Alpaca batched + Yahoo + a Kraken WS OHLC snapshot sweep), correcting the regressions where iapi was swept over the full catalog at an assumed-but-false ~40 req/s and WS streamed 12k symbols permanently. ADR-113 records cross-source bar **merge data integrity** — a trusted-tier recent-window outlier guard (the WOK 2× incident) plus the fact that Kraken sources stocks from Alpaca's backend, so Yahoo is the only independent corroborator. ADR-114 deprecates martingale live-trading support to `archive/martingale-deprecated` because position-size escalation is not acceptable for supported live trading. ADR-115 deprecates the standalone CLI/TUI to `deprecated/cli-tui`, leaving the native GUI as the active product surface. ADR-119 records that live forming bars are owned by the provider-neutral chart live-quote overlay path, not the deleted unreachable Alpaca `StartStream` trade-tick path. ADR-120 records regulatory outlier alerts rendered beside chart symbols (chart header + watchlist badges + a `REG_SHO` window), from free NasdaqTrader feeds — Reg SHO threshold securities and trading halts / LULD pauses; borrow-rate feeds are deferred as paid-only.

**2026-06 consolidation:** execution-log passes were merged into durable parents and replaced with short redirect stubs (so cross-references keep resolving, numbers stay permanent): perf/O(1) passes (060, 072, 074, 075, 076, 105) → **ADR-098** (now "Performance & O(1) Optimization Program"); GPU parity passes (041, 071) → **ADR-030**; transpiler phase 2 (068) → **ADR-067**. Full pre-merge detail remains in git history.

Thematic groups for onboarding:

- **Chart/rendering UX:** ADR-001, 002, 004, 005, 007, 016, 017, 027, 030, 048, 098, 119.
- **Broker/data sync:** ADR-003, 008, 009, 029, 036, 050, 051, 087, 094, 095, 099, 101, 102, 103, 107, 109, 110, 111, 112, 113.
- **Research/news/fundamentals:** ADR-011, 020, 034, 056, 057, 062, 063, 073, 078, 080, 096, 100, 116, 117, 120.
- **Performance/security/maintenance:** ADR-006, 031, 032, 033, 039, 044, 059, 061, 077, 088, 089, 091, 098, 108, 118.
- **Trading/workflow/AI:** ADR-013, 014, 015, 053, 082, 083, 084, 114, 115.

See the root README ADR Index for the numbered list.
