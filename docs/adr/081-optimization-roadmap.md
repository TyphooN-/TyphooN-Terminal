# ADR-081: Optimization Roadmap (2026-04-08)

**Status:** In Progress | **Date:** 2026-04-08

## Context

Comprehensive audit of the TyphooN-Terminal codebase identified 10 actionable optimization opportunities across GPU utilization, UX responsiveness, performance, and code quality. This ADR tracks each item to completion.

## Items

### GPU (High Impact)
- [ ] **Dynamic shader parameters** — MACD (12/26/9), Ichimoku (9/26/52), Parabolic SAR (0.02/0.2) hardcoded in WGSL shaders. Requires custom bind group layout for multi-param uniforms. Blocked on adding period input UI.
- [ ] **GPU health dashboard** — Track which indicators use GPU vs CPU fallback. Show compute distribution in Settings or debug panel.

### UX (Medium-High Impact)
- [ ] **Responsive window sizes** — 50+ windows use fixed `default_size([X, Y])`. Low priority: egui remembers user resize after first open.
- [x] **Right-click context menus** — Watchlist: Chart/Remove context menu (done). Positions/Orders: future.
- [x] **Sub-pane height constant** — Extracted to `SUB_PANE_H` const.
- [x] **Missing tooltips** — Added to Destroy Lines, Open MG, Close Partial. More can be added incrementally.

### Performance (Medium Impact)
- [ ] **Indicator Vec reuse** — 40+ `Vec::with_capacity` per indicator compute. Use object pool or persistent buffers.
- [x] **Crypto refresh guard** — Already gated on active tab (verified in code).

### Code Quality (Low-Medium)
- [x] **Metrics server error handling** — Replaced `.unwrap()` with `if let Err` on encode.
- [ ] **Dead BrokerCmd variants** — Document or implement UI triggers for 40+ unused enum variants.

## Consequences

Completing all items will improve: GPU utilization (dynamic parameters unlock user customization), UX polish (context menus, tooltips, responsive layout), frame time (fewer allocations), and code robustness (no unwrap in production paths).
