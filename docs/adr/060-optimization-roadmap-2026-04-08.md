# ADR-060: Optimization Roadmap (2026-04-08)

**Status:** Complete | **Date:** 2026-04-08

## Context

Comprehensive audit of the TyphooN-Terminal codebase identified 10 actionable optimization opportunities across GPU utilization, UX responsiveness, performance, and code quality. All items completed.

## Items

### GPU (High Impact)
- [x] **Dynamic shader parameters** — MACD periods (fast/slow/signal) now configurable via DragValue UI in indicator settings. Both CPU and GPU paths use user values. GPU shader uses bit-packed periods: `fast | (slow << 8) | (signal << 16)` in the existing params.period uniform — no custom bind group needed (ADR-071).
- [x] **GPU health dashboard** — GPU Indicators / GPU DARWIN Analytics status shown in Help window (Active/CPU fallback).

### UX (Medium-High Impact)
- [x] **Responsive window sizes** — Deferred: egui remembers user resize after first open. Fixed sizes are reasonable defaults.
- [x] **Right-click context menus** — Watchlist: Chart/Remove context menu.
- [x] **Sub-pane height constant** — Extracted to `SUB_PANE_H` const.
- [x] **Missing tooltips** — Added to Destroy Lines, deprecated hedge setup, Close Partial. The hedge setup was later removed from active builds by ADR-114.

### Performance (Medium Impact)
- [x] **Indicator Vec reuse** — MACD computation reuses existing Vec allocations (clear+reserve+push instead of new Vec).
- [x] **Crypto refresh guard** — Already gated on active tab.

### Code Quality (Low-Medium)
- [x] **Metrics server error handling** — Replaced `.unwrap()` with `if let Err` on encode.
- [x] **Dead BrokerCmd variants** — Documented: all variants handled in broker task, some lack dedicated UI buttons but accessible via console commands.

## Consequences

All 10 items resolved. MACD periods now user-configurable (12/26/9 defaults). GPU status visible in Help window. Vec allocations reused for indicators. Zero production unwrap/expect (ADR-061).
