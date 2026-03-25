# ADR-051: Dependency Version Alignment

**Status:** Implemented
**Date:** 2026-03-25

## Context

Multiple crates in the workspace had version splits causing duplicate compilation, slower builds, and potential security exposure from outdated transitive dependencies.

## Decision

Align all direct dependencies to their latest stable versions across the entire workspace. Eliminate version splits where the same crate was pinned at different versions in different workspace members.

## Changes (2026-03-25)

| Crate | Before | After | Notes |
|-------|--------|-------|-------|
| wgpu | 24 | **27** | Match eframe 0.33 internal version — eliminates naga 24/27 duplicate |
| rfd | 0.15 | **0.17** | Native file dialog |
| pest / pest_derive | 2.7 | **2.8** | Parser (mql5-compiler) |
| rusqlite (cli) | 0.34 | **0.39** | Match engine version — was 5 versions behind |
| ratatui | 0.29 | **0.30** | TUI framework (cli) |
| crossterm | 0.28 | **0.29** | Terminal backend (cli) |
| rand (cli) | 0.9 | **0.10** | Match engine version |

## Remaining Transitive Duplicates

These duplicates come from upstream crates and cannot be resolved without upstream updates:

- `calloop` 0.13/0.14 — Wayland compositor bindings (smithay)
- `getrandom` 0.2/0.3/0.4 — rand ecosystem transition
- `rustix` 0.38/1.1 — smithay/calloop transition
- `thiserror` 1/2 — ecosystem-wide migration
- `hashbrown` 0.15/0.16 — indexmap/hashbrown transition

## Consequences

- **Pro:** Eliminates naga duplicate (largest version split — 2 full wgpu implementations)
- **Pro:** All workspace crates on latest stable versions
- **Pro:** Reduced compile time from fewer duplicate builds
- **Pro:** Security: no known CVEs in outdated pinned versions
- **Con:** Remaining ~20 transitive duplicates are unavoidable until upstream migrates
