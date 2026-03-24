# ADR-043: GPU-First Grid Rendering

**Status:** Implemented
**Date:** 2026-03-23

> **Note:** Extends [ADR-042](042-mtf-grid-unified-indicator-pipeline.md) (Unified Indicator Pipeline) and [ADR-032](032-gpu-drawing-tools-roadmap.md) (GPU Chart Roadmap).

## Context

The MTF Grid creates 4-8 chart cells simultaneously. Each cell needs candle rendering + indicator overlays. Two approaches were considered:

1. **CPU fallback (lightweight-charts)** — full indicator support but slower rendering, higher memory
2. **GPU-first (WebGL2)** — optimal performance, but GPU engine doesn't yet support histograms, baseline fills, or sub-pane rendering (Phase 4 in ADR-032)

## Decision

**GPU is always the primary rendering engine.** CPU (lightweight-charts) is only a fallback for systems without WebGL2 support or proper GPU drivers.

### Grid Cell GPU Wrapper

Each GPU grid cell gets a full `addLineSeries()` wrapper that routes indicator data through `gpu.add_line()`:

```javascript
addLineSeries: (opts) => ({
  setData: (data) => {
    // Parse color → RGB, pack values → Float64Array, call gpu.add_line()
    gpu.add_line(values, r, g, b, 1.0);
  }
})
```

### What Works in GPU Grid Cells Now
- Candlestick/bar rendering (WebGL2)
- All line-based indicators: SMA, EMA, KAMA, DEMA, HMA, WMA, Bollinger, ATR Projection
- Fisher Transform + signal line (via dedicated lightweight-charts pane)
- BetterVolume histogram (via dedicated lightweight-charts pane)

### GPU Phase 4 — Completed (2026-03-24, ADR-046)
- `addHistogramSeries()` — now routes to `gpu.add_histogram()` with per-bar RGBA colors
- `addBaselineSeries()` — now routes to `gpu.add_fill()` with top/bottom value arrays
- GPU WASM rebuilt with `add_histogram()`, `add_fill()`, `add_pane_histogram()` exports
- Grid cell GPU wrappers fully wired — BetterVolume and S/D zone fills render on GPU

### What Requires GPU Phase 5 (Deferred)
- Price scale labels for indicator sub-panes

### Performance Architecture

```
Grid Cell Load Sequence:
1. Fetch bars (40ms via get_bars_tail)
2. setData → GPU renders candles (instant)
3. requestAnimationFrame yield (UI paints candles)
4. applyIndicators → GPU add_line per indicator
5. GPU final render (all lines composited)
6. Next cell starts
```

Cells load **sequentially** to prevent main-thread starvation from 4+ cells computing indicators simultaneously. Each cell yields via `requestAnimationFrame` between candle render and indicator computation.

## Consequences

- **Pro**: GPU rendering for all chart cells — maximum performance
- **Pro**: Unified `applyIndicators()` code path — one fix applies everywhere
- **Pro**: Fisher/Volume panes still use lightweight-charts (full feature support)
- **Con**: No price labels on GPU indicator lines until Phase 5
- **Updated 2026-03-24**: GPU Phase 4 completed — histograms and baseline fills now render on GPU (see ADR-046)
