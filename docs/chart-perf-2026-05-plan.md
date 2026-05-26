# Chart Rendering Performance & UX Status

Last reviewed: 2026-05-26

## Goal

Keep live Kraken/streaming chart updates smooth while preserving TradingView/MT5-style chart interaction.

## Current implementation

Implemented in the native app:

- `ChartState` tracks `visible_bars_gen`, `forming_bar_dirty`, `last_visible_bar_ts`, `last_rendered_gen`, and `last_rendered_bar_ts`.
- Forming-bar updates can mutate the last bar without incrementing the closed-bar generation counter.
- The forming-bar GPU path writes only the last bar into existing close/open/OHLC/mid/volume buffers when those buffers are resident, avoiding a full OHLCV upload on live ticks.
- Closed-bar or structural changes call `mark_structural_change()` and increment `visible_bars_gen`.
- Indicator computation has a forming-bar fast path that consumes `forming_bar_dirty` without forcing a full recompute.
- Empty charts skip indicator computation immediately.
- Heavy sync suppresses expensive chart indicator computation.
- Broker-drain repaint pressure is throttled with `request_repaint_after(...)` instead of forcing immediate repaint spam.
- Price-axis dragging is isolated to the right price scale and behaves like TradingView/MT5.

## Current status

The high-impact responsiveness work is complete for the current scope.

The remaining work is performance-hardening, not a correctness blocker:

- Route more indicator families through GPU compute where profiling proves the CPU path is hot.
- Add deeper render-cache/content-hash checks only if profiling shows remaining idle-frame chart work is material.

## Verification references

Recent relevant checks:

- `cargo check -p typhoon-native --quiet`
- `cargo check -p typhoon-native --tests --quiet`
- chart forming-bar/unit coverage in `typhoon-native`
- `git diff --check`

## Maintenance rule

Do not treat this file as an execution checklist. Keep it as an architecture/status record. New chart-performance work should land as code plus focused tests, then update this file only when the architecture or remaining risk changes.
