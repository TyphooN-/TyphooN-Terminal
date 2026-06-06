# ADR-108: Research Module Compile-Time Modularization

**Status:** Accepted
**Date:** 2026-06-06

## Context

`typhoon-engine::core::research` had become the dominant engine compile-time and rust-analyzer hot spot. Before this ADR, `engine/src/core/research/mod.rs` contained roughly 90k lines, including public DTOs, constants, provider fetchers, SQLite helpers, and many compute surfaces.

Measured before the first split:

- Warm `cargo check -p typhoon-engine`: about 9.4s.
- Touching a small engine helper previously cost about 11.5s.
- Touching the research monolith previously cost about 11.9s.
- `engine/src/core/research/mod.rs`: 90,269 lines.

The terminal already uses `mold` and high parallelism, so the next useful compile-speed work is structural: reduce the blast radius of the research module and eventually isolate research from broker/storage edit loops.

## Decision

Split research in verified increments while preserving the public API through root re-exports.

Initial structure:

- `engine/src/core/research/mod.rs`
  - orchestration, existing fetch/cache/compute code not yet extracted
  - `pub use` re-exports for extracted modules
- `engine/src/core/research/types.rs`
  - public research DTOs and constants formerly at the top of `mod.rs`
- `engine/src/core/research/technical.rs`
  - TECH compute surface (`compute_technical_indicators`) and direct dependencies

Rules for future slices:

1. Move cohesive feature families, not arbitrary line ranges.
2. Preserve public names via `pub use` from `mod.rs`.
3. Run `cargo check -p typhoon-engine` after each extraction.
4. Run downstream `cargo check -p typhoon-native` before committing a migration slice.
5. Prefer extracting research/provider/storage crates only after module boundaries are stable enough to avoid circular dependencies.

## Follow-up Plan

Next structural targets, in order:

1. Continue splitting research compute families into semantic modules:
   - valuation/risk composites
   - market/seasonality/correlation surfaces
   - TA/indicator parity surfaces
   - SQLite schema/helper families
2. Extract shared lightweight domain types to a small crate only when needed to break cycles.
3. Extract `typhoon-research` once dependencies on `crate::core::fundamentals` and `crate::core::sec_filing` have been inverted or moved to shared crates.
4. Keep broker/cache hot paths out of the research crate so a Kraken/Alpaca sync edit does not invalidate heavy research code.
5. Evaluate `sccache` only when installed/configured on the machine; do not set `rustc-wrapper` to a missing binary.
6. Keep `tokio-tungstenite` TLS cleanup gated behind LAN-sync verification, because LAN sync currently uses native-tls self-signed certificate handling.

## Consequences

Positive:

- DTO/constant edits no longer require editing the root 80k+ line research file.
- TECH compute edits are isolated into a small module.
- The public API remains compatible for downstream callers.
- This creates a safer path toward a future `typhoon-research` crate.

Tradeoffs:

- This first slice is not enough to solve engine-wide invalidation by itself.
- `mod.rs` remains large and needs more semantic extractions.
- Crate extraction is deferred until dependency cycles are resolved deliberately rather than by whack-a-mole call-site rewrites.
