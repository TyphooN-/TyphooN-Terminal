# ADR-081: Optimization Roadmap (2026-04-08)

**Status:** In Progress | **Date:** 2026-04-08

## Context

Comprehensive audit of the TyphooN-Terminal codebase identified 10 actionable optimization opportunities across GPU utilization, UX responsiveness, performance, and code quality. This ADR tracks each item to completion.

## Items

### GPU (High Impact)
- [ ] **Dynamic shader parameters** — MACD (12/26/9), Ichimoku (9/26/52), Parabolic SAR (0.02/0.2) hardcoded in WGSL shaders. Params struct exists but values ignored. Enable user-configurable GPU indicators.
- [ ] **GPU health dashboard** — Track which indicators use GPU vs CPU fallback. Show compute distribution in Settings or debug panel.

### UX (Medium-High Impact)
- [ ] **Responsive window sizes** — 50+ windows use fixed `default_size([X, Y])`. Scale to viewport percentage for 4K/small screens.
- [ ] **Right-click context menus** — Watchlist: chart it, remove. Positions: close, modify SL/TP. Orders: cancel.
- [ ] **Sub-pane height constant** — Hardcoded 80.0px appears 100+ times. Extract to const or make responsive.
- [ ] **Missing tooltips** — 50+ buttons without `.on_hover_text()`. Batch add.

### Performance (Medium Impact)
- [ ] **Indicator Vec reuse** — 40+ `Vec::with_capacity` per indicator compute. Use object pool or persistent buffers.
- [ ] **Crypto refresh guard** — Polls every 60s even when crypto chart isn't visible. Gate on active tab.

### Code Quality (Low-Medium)
- [ ] **Metrics server error handling** — Replace `.unwrap()` with proper error handling in metrics.rs.
- [ ] **Dead BrokerCmd variants** — Document or implement UI triggers for 40+ unused enum variants.

## Consequences

Completing all items will improve: GPU utilization (dynamic parameters unlock user customization), UX polish (context menus, tooltips, responsive layout), frame time (fewer allocations), and code robustness (no unwrap in production paths).
