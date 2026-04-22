# ADR-199: Parity Expansion R96 — INSIDERCONC

**Date:** 2026-04-22  
**Status:** Accepted  
**Related:** `engine/src/core/research.rs`, `engine/src/core/lan_sync.rs`, `native/src/app.rs`, ADR-127, ADR-198

## Context

After ADR-198, the only named Godel parity holdout left in the ADR trail
was `INSIDERCONC`.

That surface had been deferred for multiple rounds because the cache did
not expose a direct `Fundamentals.insiders_percent_held` field. What the
terminal does have now is enough to derive a practical equivalent:

- cached `INS` rows from Form 4 / FMP insider-trade fetches
- `shares_owned_after` on each insider row
- `Fundamentals.shares_outstanding` for normalization

That does not produce a perfect vendor-grade insider ownership feed, but
it does produce a transparent, cache-backed estimate that fits the parity
arc far better than leaving the last surface permanently blocked.

## Decision

Implement `INSIDERCONC` as a derived sector-rank surface:

- for each symbol, keep the latest known `shares_owned_after` per insider
  reporter from cached `INS` rows
- sum those latest holdings into an estimated insider-held share count
- divide by `Fundamentals.shares_outstanding` to get estimated insider-held %
- rank that estimated percentage vs same-sector peers

Ship the usual parity wiring in one pass:

- engine compute + snapshot type
- SQLite persistence table `research_insiderconc`
- LAN sync registration
- research-packet output
- palette aliases
- broker command / result plumbing
- egui popup

## Consequences

- The named `INSIDERCONC` parity gap is now closed without inventing a
  new upstream cache or vendor dependency.
- The metric is explicitly an estimate derived from the latest cached
  insider holdings per reporter, not a canonical ownership field.
- Coverage depends on `INS` cache quality. Symbols with sparse insider
  filings still fall back to `NO_DATA` / `INSUFFICIENT_DATA`.
- This is complementary to `SHRANK`: `SHRANK` ranks pessimistic short
  positioning, while `INSIDERCONC` ranks insider ownership concentration.

## Validation

- `cargo test --manifest-path engine/Cargo.toml insiderconc -- --nocapture`
- `cargo check --manifest-path native/Cargo.toml`
