# Architecture Decision Records

This directory is intentionally compacted for human onboarding.

Rules:

1. One ADR per durable architectural decision, not per work session.
2. No more numbered parity rounds or execution-log ADRs.
3. If work follows an existing architecture, update the existing ADR instead of creating a new one.
4. Historical per-round detail lives in git history; the top-level ADR set should stay readable.

Current count: 96 ADRs (removed-feature ADRs — Darwin/MT5/Tastytrade/CryptoCompare/LAN/web — were deleted 2026-06; numbers are not reused, so gaps in the sequence are expected). ADR-111 records the broker & data-source scope reduction to **Kraken + Alpaca only** — Darwin/Darwinex, MT5/BarCacheWriter, Tastytrade, and CryptoCompare are deprecated to `deprecated/*` branches (preserved as restore points, not built or maintained in the interim). ADR-112 records the equities bar-sync split into a **demand-depth lane** (Kraken WS live + iapi, demand-scoped) and a **catalog-breadth lane** (Alpaca batched + Yahoo + a Kraken WS OHLC snapshot sweep), correcting the regressions where iapi was swept over the full catalog at an assumed-but-false ~40 req/s and WS streamed 12k symbols permanently. ADR-113 records cross-source bar **merge data integrity** — a trusted-tier recent-window outlier guard (the WOK 2× incident) plus the fact that Kraken sources stocks from Alpaca's backend, so Yahoo is the only independent corroborator. ADR-114 deprecates martingale live-trading support to `archive/martingale-deprecated` because position-size escalation is not acceptable for supported live trading. ADR-115 deprecates the standalone CLI/TUI to `deprecated/cli-tui`, leaving the native GUI as the active product surface.

**2026-06 consolidation:** execution-log rounds were merged into durable parents and replaced with short redirect stubs (so cross-references keep resolving, numbers stay permanent): perf/O(1) passes (060, 072, 074, 075, 076, 105) → **ADR-098** (now "Performance & O(1) Optimization Program"); GPU parity rounds (041, 071) → **ADR-030**; transpiler phase 2 (068) → **ADR-067**. Full pre-merge detail remains in git history. (Further index thematic-grouping is a pending follow-up.)

See the root README ADR Index for the numbered list.
