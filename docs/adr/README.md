# Architecture Decision Records

This directory is intentionally compacted for human onboarding.

Rules:

1. One ADR per durable architectural decision, not per work session.
2. No more numbered parity rounds or execution-log ADRs.
3. If work follows an existing architecture, update the existing ADR instead of creating a new one.
4. Historical per-round detail lives in git history; the top-level ADR set should stay readable.

Current count: 111 ADRs. ADR-111 records the broker & data-source scope reduction to **Kraken + Alpaca only** — Darwin/Darwinex, MT5/BarCacheWriter, Tastytrade, and CryptoCompare are deprecated to `deprecated/*` branches (preserved as restore points, not built or maintained in the interim).

See the root README ADR Index for the numbered list.
