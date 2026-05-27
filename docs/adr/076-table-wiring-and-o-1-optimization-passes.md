# ADR-076: Table Wiring and O(1) Optimization Passes

**Status:** Implemented
**Date:** 2026-04-12
**Compacted:** 2026-05-27
**Supersedes:** Old ADR 099 through old ADR 105

## Context

The old ADR set split a single table-wiring/performance cleanup campaign into seven separate pass records. That was execution journaling, not durable architecture. This compact ADR keeps the architectural decisions and removes the pass-by-pass noise from the top-level ADR directory.

## Decision

- Symbol-bearing tables should share common context-menu behavior instead of bespoke per-window click handlers.
- Small inline visualizations such as sparklines should reuse cached bars and prefetch visible rows only.
- Hot paths should avoid accidental O(n²), repeated `String` allocation, and avoidable HashMap/Vec reallocations.
- Borrow-checker workarounds belong in small pending-action structs or post-closure application points, not in duplicated UI logic.

## Compacted implementation inventory

| Old ADR | Compacted topic | Implementation slices |
| --- | --- | --- |
| 099 | Full Table Wiring Pass: Context Menus, Sparklines, Arc\<str\> Interning | UX3: Right-Click Symbol Context Menu — Wired into 9 Tables; UX7: Inline Sparklines — Wired into 3 Outlier Tables + EV Scanner; PERF5: Arc\<str\> in detect_outliers |
| 100 | Deeper Table Wiring + Workspace Presets + Sparkline LRU | UX7: Sparklines in 5 Tables (was 3); UX3: Right-Click Menus in Live Positions/Orders (was 9 tables); UX4: Built-In Workspace Presets (4 named layouts); MEM: Sparkline Cache Soft Cap |
| 101 | Deeper Wiring Pass 2: Live Orders, Congress, Fundamentals Sparkline, Hot-Path Clones | UX3: Context Menus in 2 More Tables (was 11 tables, now 13); UX7: Sparklines in Fundamentals Window; PERF: Eliminated Redundant Symbol Clones in OUTLIERS Handlers |
| 102 | Deeper Wiring Pass 3: Calendars + Insider Window + active_symbols Cache | UX3: Context Menus in 4 More Tables (now 17 total); UX7: Sparkline in Insider Trades Window; PERF: Per-Frame Cached active_symbols + O(1) Dedup |
| 103 | Deeper Wiring Pass 4: Cached Scoped Fundamentals + Stat Arb + .to_uppercase Elimination | PERF: Per-Frame Cached scoped_fundamentals_owned; PERF: Eliminated Per-Row .to_uppercase() in EV Scanner Hot Path; UX3: Stat Arb Pairs Context Menus (now 18 surfaces) |
| 104 | Deeper Wiring Pass 5: O(1) Active Filter, Filing Truncation, Backfill Menu | PERF: O(n²) → O(1) Active Symbol Filter; MEM: Filing Content Truncation Cap; UX3: Crypto Backfill Grid Context Menu (now 19 surfaces) |
| 105 | Deeper Wiring Pass 6: BG Blacklist HashSet, Pre-Allocated Caches | PERF: BG Thread Blacklist Vec → HashSet; PERF: Pre-Allocated Cache Capacities; Investigation: build_trade_overlay String::contains |

## Maintenance rule

Future table wiring or O(1) cleanup should update this ADR when it follows the same architecture. Create a new ADR only for a new reusable UI primitive, cache contract, or scheduling model.
