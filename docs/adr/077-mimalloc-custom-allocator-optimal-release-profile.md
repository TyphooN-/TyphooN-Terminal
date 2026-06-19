# ADR-077: mimalloc Custom Allocator + Optimal Release Profile

**Status:** Implemented
**Date:** 2026-04-12

## Context

After extensive UI-level optimization (ADRs 097-105), the next class of wins
is at the runtime infrastructure level. A research pass identified two
high-impact opportunities:

1. **Custom allocator**: TyphooN's render loop allocates heavily (per-frame
   Strings, Vecs, HashMaps for window state, sparkline caches, scope filters).
   The system allocator (glibc malloc) is suboptimal for this small-allocation
   pattern.

2. **Release profile**: Was using `opt-level=2` and `lto="thin"`, leaving
   significant compiler optimization on the table.

## Implemented

### Custom Allocator: mimalloc
- Added `mimalloc = { version = "0.1", default-features = false }` to
  typhoon-native/Cargo.toml.
- Wired as global allocator in main.rs:
  ```rust
  #[global_allocator]
  static GLOBAL: mimalloc::MiMalloc = mimalloc::MiMalloc;
  ```
- mimalloc is Microsoft Research's allocator — optimized for small-object
  allocation (< 1KB) which dominates UI rendering workloads.
- Expected impact: **5-15% reduction in frame latency** for render-heavy
  operations (egui UI updates, plot data, sparkline rendering).
- Memory cost: ~50KB additional binary size (worth it for the win).

### Optimal Release Profile
Cargo.toml [profile.release]:
```toml
[profile.release]
opt-level = 3        # max optimizations (was 2)
lto = "fat"          # full-program LTO (was thin) — better cross-crate inlining
codegen-units = 1    # single codegen unit for max optimization
strip = true         # already enabled
panic = "abort"      # smaller binary, no unwind tables (NEW)
debug = false        # no debug info (NEW)
```

**Trade-offs:**
- Build time: ~3 minutes (debug) → ~8 minutes (release with LTO=fat). Acceptable
  for production builds.
- Binary size: smaller due to `panic=abort` removing unwind tables and
  `strip=true` removing symbols.
- Runtime performance: **opt-level=3 enables aggressive auto-vectorization,
  loop unrolling, and inlining**. `lto=fat` allows cross-crate inlining
  across engine ↔ native. `codegen-units=1` forces single-pass compilation
  for maximum optimization.

## GPU Shader Findings (Not Implemented — Cost > Benefit)

The research also identified GPU shader optimizations:

- **DARWIN_CORR shared memory**: Would require restructuring the tile dispatch
  pattern. Each thread reads from a different (i,j) pair, so the data isn't
  naturally tile-aligned. The estimated 10-20× speedup is unrealistic for
  this access pattern.
- **Sequential indicator shaders** (EMA, RSI, MACD, etc.): Inherently
  sequential due to dependencies on previous bar state. Cannot parallelize
  without changing the algorithm. Accept as-is.
- **Pipeline caching**: Marginal startup-time-only win. Defer.
- **Const-genericized indicators**: Code duplication cost > 5-8% speedup gain
  for fixed periods.

## Compile-Time Codegen Findings (Not Implemented — Marginal)

- **COMMANDS binary search**: 245 entries, called per-keystroke. Linear scan
  takes ~3μs, binary search would take ~0.5μs. Saves <1ms per session.
  Not worth proc-macro complexity.
- **Macro → const tables**: Quality improvement, not runtime win.

## Tests

904 tests pass with new profile + mimalloc. Zero warnings. Zero production
unwrap/expect violations.

## Files Changed

- `Cargo.toml` — release profile flags
- `typhoon-native/Cargo.toml` — mimalloc dependency
- `typhoon-native/src/main.rs` — global allocator declaration
