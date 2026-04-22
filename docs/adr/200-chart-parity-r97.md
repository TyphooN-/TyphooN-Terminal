# ADR-200: Chart Parity R97 — GPU/CPU Chart Indicators For CMO, QSTICK, DISPARITY, BOP, STDDEV

**Date:** 2026-04-22  
**Status:** Accepted  
**Related:** `native/src/app.rs`, `native/src/gpu_compute.rs`, ADR-188, ADR-199

## Context

ADR-188 deferred chart-drawing parity after the research/cache surface work
had moved far ahead of the chart UI. By ADR-199 the named research holdouts
were effectively closed, but a practical parity gap remained:

- several imported TA-Lib-style surfaces existed in research windows only
- the chart pane system still could not display them directly
- recent user guidance tightened the bar further: newly imported TA-Lib
  indicators should land as chart indicators with GPU compute and CPU fallback

The clean next slice was the existing chartable oscillator/stat bundle already
present in research:

- `CMO`
- `QSTICK`
- `DISPARITY`
- `BOP`
- `STDDEV`

These fit the current sub-pane renderer without reopening the much larger
candlestick-marker overlay problem.

## Decision

Reopen chart parity through a GPU-backed sub-pane bundle:

- add `CMO(9)`, `QStick(14)`, `Disparity(14)`, `BOP(14)`, and `StdDev(20)`
  to `ChartState`
- compute them during chart indicator refresh with GPU-first execution and
  CPU fallback
- add dedicated WGSL compute shaders for the bundle in `gpu_compute.rs`
- expose them through both indicator menus, session/template persistence,
  preset/reset paths, data-window readout, and dedicated chart-toggle commands

Command aliases are chart-specific (`*_CHART` / `SHOW_*`) so the existing
research-window commands (`CMO`, `QSTICK`, `DISPARITY`, `BOP`, `STDDEV`)
keep their current behavior.

## Consequences

- The chart parity track is active again, but through chartable indicator
  bundles rather than per-bar annotation overlays.
- Imported TA-Lib-style indicators in this bundle now satisfy the expected
  GPU-first, CPU-fallback chart behavior.
- Session restores, templates, presets, and menu toggles now keep the new
  panes in sync with the existing chart indicator model.
- Candlestick-pattern overlays remain separate future work under ADR-188.

## Validation

- `cargo test --manifest-path native/Cargo.toml chart_talib_gpu_fallback_series_have_expected_ranges -- --nocapture`
- `cargo test --manifest-path native/Cargo.toml template -- --nocapture`
- `cargo check --manifest-path native/Cargo.toml`
