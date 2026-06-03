# ADR-108: Research Module Decomposition for Compile Speed

**Date:** 2026-06-03
**Status:** Accepted
**Related:** `engine/src/core/research.rs`, `engine/src/core/research/`, ADR-079, ADR-086, ADR-105

## Context

`engine/src/core/research.rs` grew into a ~90k-line mixed module containing:

- public research DTOs and cached-domain types;
- Finnhub/FMP/Yahoo/CFTC fetch and parse code;
- SQLite schema, upsert, and read helpers;
- Godel parity feature snapshots and rankings;
- TA-style technical indicator snapshots.

That made research edits painful to review and contributed to slow engine/native rebuilds, especially near the final `typhoon` binary build step. The module also had weak ownership boundaries: provider IO, storage, Godel feature math, and technical indicators lived in one file even when they changed for unrelated reasons.

## Decision

Convert `research.rs` into a directory module and split by feature family, not by arbitrary line ranges.

Current target layout:

```text
engine/src/core/research/
├── mod.rs         # root API and remaining migration surface
├── types.rs       # shared DTOs / serializable research domain types
├── godel.rs       # Godel parity features, rankings, and related storage
├── technical.rs   # TA-style RSI/MACD/BB/ATR/ADX/Stochastic snapshot logic
├── finnhub.rs     # Finnhub provider code, when extracted
├── fmp.rs         # Financial Modeling Prep provider code, when extracted
├── yahoo.rs       # Yahoo/options/cross-asset provider code, when extracted
├── cache.rs       # generic SQLite cache helpers, when extracted
└── cftc.rs        # CFTC COT reports, when extracted
```

The root module keeps a flat public API with `pub use` re-exports so native/UI callers do not need a migration at the same time as the mechanical split.

## Split Rules

- Move cohesive domains only: types, technical indicators, options/IV/skew, returns/risk, fundamentals, rankings, providers, storage.
- Do not keep splitting `godel.rs` merely because it is large. Split out non-Godel concerns first.
- Keep Godel parity feature families together unless a subdomain has an obvious independent owner.
- Run `cargo check -p typhoon-engine` after every extraction.
- Run `cargo test -p typhoon-engine --lib` after non-trivial moves.

## Initial Implementation

- `research.rs` was replaced by `research/mod.rs` plus submodules.
- shared research DTOs moved to `types.rs`.
- Godel parity feature and storage blocks moved to `godel.rs`.
- TA-style technical indicators moved to `technical.rs`:
  - `compute_technical_indicators`
  - `upsert_technicals`
  - `get_technicals`

Verified locally:

```text
cargo check -p typhoon-engine
cargo test -p typhoon-engine --lib --quiet
```

## Consequences

Positive:

- Smaller edit surfaces for research work.
- Clearer ownership for Godel parity vs. TA-style indicators vs. providers/storage.
- Safer future extraction path because each move has a semantic boundary.

Tradeoff:

- The public API remains flat during migration, so submodule internals still rely on `super::*` in places. That is acceptable during the mechanical split; imports can be narrowed after the module tree stabilizes.

## Next Good Splits

1. `options.rs`: Yahoo options chain, IV rank/percentile, volatility skew, calendar put/call behavior.
2. `returns.rs` / `risk.rs`: HRA, total return, realized volatility, drawdown, return distribution stats.
3. `fundamentals.rs`: DCF, SVM, FCF yield, leverage, accruals, margins, Piotroski, Altman.
4. `rankings.rs`: VAL/QUAL/RISK ranks and Godel relative-rank surfaces.
5. provider files: Finnhub, FMP, Yahoo, CFTC fetch/parse code.
6. `storage.rs`: SQLite schema/upsert/get helpers once feature modules no longer need broad shared access.
