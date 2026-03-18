# ADR-030: Session Persistence, Security Hardening & Feature Expansion (112 Commands)

**Status:** Implemented
**Date:** 2026-03-18

## Context

In a single session, the terminal grew from 39 to 151+ Ctrl+K commands across 6 waves. This ADR documents the architectural changes that supported this expansion without degrading reliability.

## Session Persistence (Enhanced)

The session save/restore system was expanded to preserve the full user experience across restarts:

| State | Before | After |
|---|---|---|
| Tabs (symbol + TF) | ✅ Saved | ✅ Saved |
| Indicators | ✅ Saved | ✅ Saved |
| Pane heights | ✅ Saved | ✅ Saved |
| Chart type | ❌ Lost | ✅ Saved |
| Panel collapse states | ❌ Lost | ✅ Saved |
| MTF grid (active + TFs) | ❌ Lost | ✅ Restored on restart |
| Chart zoom/scroll position | ❌ Lost | ✅ Restored (deferred apply) |
| Order type selector | ❌ Lost | ✅ Saved |
| Last news article | ❌ Lost | ✅ Re-opened on restart |
| Theme | ✅ Saved | ✅ Saved (via DARKMODE) |

## Duplicate Fetch Elimination

`loadMTFData()` was populating `mtfData` but NOT `barCache`. The prefetch system didn't see these as cached, causing double-fetches for H1/H4/D1/W1 on every chart load. Fix: `loadMTFData` now writes to both `mtfData` and `barCache`.

## Pre/Post-Market Pricing

Switched stock quote endpoint from `/v2/stocks/{symbol}/quotes/latest` to `/v2/stocks/{symbol}/snapshot`. The snapshot includes `latestTrade` which captures extended hours trades on IEX (free). Dashboard shows orange "Last: $X @ HH:MM" for extended hours vs normal "Bid/Ask/Spread" during regular session. Last known price persisted in localStorage per symbol.

## Security Pass 20

| Fix | Category |
|---|---|
| innerHTML → createElement in 10 functions | XSS prevention |
| Webhook fetch() + AbortController (5s timeout) | DoS prevention |
| Webhook URL: HTTPS-only, block localhost/private IPs | SSRF prevention |
| localStorage caps: 100 timers, 500 journal, 5MB imports | Resource limits |
| 41 serde_json unwrap → map_err | Panic prevention |
| URL path injection guard on activity_types | Path traversal |
| Indicator try/catch isolation | Cascade prevention |

## Crate Security Rollup

All dependencies updated to latest: rusqlite 0.39, reqwest 0.13, rand 0.10, tokio-tungstenite 0.29. Zero outdated.

## Feature Waves Summary

| Wave | Commands | Focus |
|---|---|---|
| 1 | 16 | Options analytics, chart tools, market analysis |
| 2 | 20 | Volume profile, risk tools, market profile, order builder |
| 3 | 20 | Dashboards, DOM, automation, themes, journaling |
| 4 | 20 | Trading tools, visual screener, tutorials, macros |
| 5 | 20 | Tax lots, PDT, voice, workspace, intermarket |
| 6 | 16 | Chart modes, AI strategy, depth chart, risk dashboard |
| **Total** | **112** | **151 total commands** |

## Architectural Principle

All 112 new features use existing cached data. Only one new API endpoint was used (stock snapshot for pre/post-market). Zero new Rust/Tauri commands added. This keeps the backend lean and avoids rate limit pressure.

## Next Steps

1. **Modular split** — 23.7K lines in one file needs ES module refactoring
2. **Smoke test** — Automated test calling all 151 commands
3. **Loading performance** — Defer non-critical command definitions
