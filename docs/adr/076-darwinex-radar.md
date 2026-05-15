# ADR-076: DarwinexRadar — Symbol Spec Change Tracking & Export

**Status:** Implemented | **Date:** 2026-04-07

## Context

Darwinex periodically changes symbol specifications: delisting instruments, adjusting swap rates, changing trade modes (e.g., close-only), modifying spreads. Traders need to detect these changes to avoid holding positions in instruments going close-only or with deteriorating swap conditions.

## Decision

DarwinexRadar compares current symbol specs against a previous snapshot stored in KV cache (`darwin:radar_prev`):
- **Change detection**: New symbols, removed symbols, swap changes, spread changes, trade mode changes (close-only alerts)
- **Changelog UI**: Collapsible section in DARWINEXRADAR window with color-coded change types
- **Export**: Semicolon-delimited CSVs to `/home/typhoon/git/typhoon-darwinex-radar/` for web publishing
- **Storage**: Previous snapshot persisted in KV, updated on each radar run

## Consequences

- Automatic detection of close-only transitions prevents trapped positions
- Historical changelog enables trend analysis on broker offerings
- Web-compatible export provides public transparency on Darwinex symbol changes
- Depends on `load_all_specs()` merging specs across all MT5 accounts

See also: ADR-057 (Symbol Specs), ADR-075 (SwapHarvester)
