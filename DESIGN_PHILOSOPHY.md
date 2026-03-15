# TyphooN-Terminal Design Philosophies

## 1. Every API Call is an Investment

API requests are a finite resource (200/min on Alpaca free plan). Each call should produce lasting value:

- **Never re-fetch historical data** — once fetched, bars are cached permanently across three tiers (memory, IndexedDB, zstd files)
- **Cache-first display** — show stale data instantly, refresh in background. The user sees a chart in milliseconds, not seconds
- **Pre-fetch aggressively** — after loading one timeframe, silently cache all others. Future timeframe switches are instant
- **Stale detection** — stop fetching the moment the API returns data we already have
- **Rate budgeting** — centralized limiter ensures no call is wasted on a 429 error

## 2. Respect Every Cycle

CPU, memory, disk, and network are all finite resources:

- **zstd compression** for cold cache — store 10x more data per byte of disk
- **IndexedDB over localStorage** — 50MB+ quota instead of 5-10MB, structured access
- **Delta updates** — only update DOM elements when values actually change (dashboard caching pattern from MQL5)
- **Indicator clipping** — don't calculate or render past the last bar
- **Rate limiter is shared** — multiple tabs, pre-fetch, and polling share one budget instead of competing

## 3. Visual Accuracy is Non-Negotiable

This is a visual system for manual trading decisions. Every indicator must look identical to MT5:

- **Exact MQL5 colors** — every color comes from the MQL5 source code defaults (clrWhite, clrMagenta, clrTomato, etc.)
- **Exact algorithms** — KAMA uses PRICE_OPEN, Fisher uses PRICE_MEDIAN, ATR period 14. All parameters match the MQL5 defaults
- **Exact line styles** — width 2 for KAMA, solid for ATR projection, dotted for grid
- **MTF filtering** — only show timeframes higher than current chart (H1 chart shows H4/D1/W1, not M15)
- **Separate panes** — Fisher and BetterVolume get their own chart windows, not overlays

## 4. Fail Gracefully, Never Destructively

- **429 rate limit** → return partial data, trigger cooldown, continue operating. Never crash or lose data
- **Late async load** → discard if tab changed. Never overwrite the wrong chart
- **Cache miss** → fall through to next tier. Never show an error for missing cache
- **API error** → log and continue. Never block the UI
- **Invalid input** → validate in Rust backend before sending to broker. Never place an invalid order

## 5. Security by Default

- **CSP enabled** — scripts restricted to self-origin only
- **No innerHTML** — all DOM updates via createElement + textContent (XSS prevention)
- **Input validation** — symbol, quantity, and side validated in Rust before any API call
- **Devtools opt-in** — only available with `--features devtools` flag, not in release builds
- **Credentials sandboxed** — Tauri webview localStorage is isolated per application

## 6. Document Everything

- **ADRs** — every significant architectural decision has a written record with context, rationale, and consequences
- **INDICATOR_PORTING.md** — lessons learned porting MQL5 indicators, with workarounds for each limitation
- **ARCHITECTURE.md** — why Rust/Tauri was chosen over Python, Electron, Qt, pure Rust GUI
- **Commit messages** — describe what changed AND why. No "fix" or "update" without context
- **No AI attribution** — commits reflect the work, not the tool

## 7. The MQL5 Heritage

TyphooN-Terminal is a port, not a rewrite. Every feature traces back to the MQL5 EA (v1.420):

- **4 order modes** (Standard, Fixed, Dynamic, VaR) — same enum, same formulas, same defaults
- **Forward-looking TRIM** — same margin math, same impossibility-of-overshoot guarantee
- **PROTECT with urgency scaling** — same `ceil(hedgeLots × urgency)` formula
- **10-button panel** — same layout, same keyboard shortcuts
- **11-label dashboard** — same metrics, same update logic
- **NNFX indicators** — same algorithms, same colors, same parameters

The Rust code mirrors the MQL5 code intentionally. A contributor familiar with `TyphooN.mq5` should recognize every function in `margin.rs`, `risk.rs`, and `martingale.rs`.

## 8. Open Source as a Product

TyphooN-Terminal aims to be a community-driven alternative to proprietary trading terminals:

- **GPL-3.0 license** — free to use, modify, and distribute
- **Self-contained** — single binary, no external services (except Alpaca API)
- **Broker-agnostic architecture** — adding a new broker means implementing one Rust trait
- **Indicator-agnostic architecture** — adding a new indicator means writing one JS function
- **Godel Terminal vision** — long-term goal of a professional-grade open-source trading terminal
