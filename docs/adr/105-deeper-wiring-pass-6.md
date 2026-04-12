# ADR-105 — Deeper Wiring Pass 6: BG Blacklist HashSet, Pre-Allocated Caches

**Status:** Implemented
**Date:** 2026-04-12

## Context

6th iteration. Searching for remaining O(n²) patterns and allocation hot spots.

## Implemented

### PERF: BG Thread Blacklist Vec → HashSet
- `darwin:deleted` blacklist filter built as `Vec<String>` then used in
  `data.accounts.retain(|a| !blacklist.contains(&a.darwin_ticker))` — that's
  O(N×M) where N=accounts, M=blacklisted.
- Converted to `HashSet<String>` for O(1) retain check.
- Marginal at 6-DARWIN scale but principled: never accept O(n²).

### PERF: Pre-Allocated Cache Capacities
- `sparkline_cache`: `HashMap::with_capacity(256)` (was empty)
- `sector_interner`: `HashMap::with_capacity(64)` (was empty)
- `cached_active_symbols`: `Vec::with_capacity(64)` (was empty)
- `cached_active_symbols_set`: `HashSet::with_capacity(64)` (was empty)
- Eliminates ~3-4 reallocation steps during initial population (HashMap
  doubles from 0 → 4 → 8 → 16 → ... → target size).
- Memory cost: ~12 KB total for all 4 maps. Worth it for the smoother
  startup ramp.

### Investigation: build_trade_overlay String::contains
- Found `entry.2.contains(&ticker)` in build_trade_overlay (lines 14605, 14619).
- This is `String::contains(&str)` for substring check on a comma-separated
  list, NOT `Vec::contains` — correct semantics for the use case.
- At typical 6-DARWIN scale, the overhead is negligible.
- No change needed.

## Tests

904 tests pass. Zero warnings. Zero production unwrap/expect violations.
