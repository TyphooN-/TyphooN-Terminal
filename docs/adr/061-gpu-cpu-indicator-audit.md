# ADR-061: GPU/CPU Indicator Audit — Parity Verification

**Status:** Implemented | **Date:** 2026-03-28 | **Updated:** 2026-03-30

## Context

Several GPU compute shaders were using different algorithms than their CPU fallbacks, producing incorrect results when the GPU path was active. An audit of all GPU indicators was performed to verify parity. The codebase now has 33 chart indicator GPU shaders + 2 DARWIN + 5 backtest = 40 total pipelines.

## Issues Found and Fixed

### Supply/Demand Zones (Critical)
- **GPU**: Old impulse-based algorithm (range > 2x avg, fixed thresholds)
- **CPU**: New fractal-based algorithm (1:1 port of SupplyDemand.mqh)
- **Fix**: GPU shader rewritten for fractal detection (parallel per-bar, 5-bar lookback). CPU handles zone testing, merging, and break purging (sequential). Hybrid GPU+CPU approach.

### OBV (On-Balance Volume)
- **GPU**: Used `abs(price_change)` as volume proxy (no volume buffer in VRAM)
- **CPU**: Used actual `bars[i].volume`
- **Fix**: Added volume buffer to GPU upload (`upload_bars_full` now accepts volumes). OBV shader receives interleaved `[close, volume]` buffer with real data.

### BetterVolume
- **GPU (original)**: Simplified ratio thresholds (vol > 2x avg = climax, etc.)
- **CPU**: Full 1:1 port of BetterVolume.mqh (buy/sell pressure estimation, lookback extremes, 2-bar analysis)
- **Fix (initial)**: CPU-only computation.
- **Fix (2026-03-30)**: Full Emini-Watch algorithm rewritten as WGSL shader with `[O,H,L,C,V]` interleaved input (5 floats/bar). GPU now achieves 1:1 parity with CPU/MQL5. BetterVolume is fully GPU-accelerated.

## Verified Matching (No Issues)

All other GPU/CPU pairs produce identical results:
- SMA(200/100), EMA(21), KAMA(10,2,30), WMA(20), HMA(20)
- Bollinger(20,2), RSI(14), MACD(12,26,9), Stochastic(14,3,3)
- ATR(14), ADX(14), CCI(20), Williams%R(14), Momentum(10)
- Fisher(32), Ichimoku(9,26,52), Parabolic SAR
- Fractals (Bill Williams, 2-bar lookback — GPU returns price, CPU returns bool, conversion verified)
- All 8 Ehlers DSP indicators (SuperSmoother, Decycler, ITL, MAMA/FAMA, EBSW, CyberCycle, CG, Roofing)
- Anchored VWAP (GPU per-day dispatch with CPU deviation bands)

### Remaining CPU-Only Indicators

| Indicator | Reason |
|-----------|--------|
| Previous Candle Levels | Day-boundary detection requires timestamps (not in GPU buffers) |
| Auto Fibonacci | O(n) argmax over fractal results — GPU dispatch overhead exceeds computation |
| Harmonic Patterns | Deeply branching 5-point XABCD geometry matching |

## New Chart Features (2026-03-30)

- **Fair Value Gaps (FVG)**: Automatic detection and rendering of price imbalance zones
- **Market Structure Labels**: Swing high/low labels with Break of Structure (BOS) and Change of Character (CHoCH) annotations
- **Volume Profile upgrade**: Enhanced histogram rendering with POC, value area, and configurable row count

## Test Coverage

480 tests (75 compiler + 319 engine + 86 native) across engine, GPU shaders, integration, and MQL5 compiler (up from 261 at initial audit).

## Consequences

- **Pro**: All indicators now produce correct, consistent results regardless of GPU availability
- **Pro**: GPU acceleration maintained for 33 of 36 chart indicators (BetterVolume + VWAP now fully GPU-wired)
- **Pro**: CPU fallback path verified functional for all indicators
- **Pro**: FVG, market structure, and Volume Profile expand chart analysis capabilities
- **Con**: S/D zone testing/merging remains CPU (inherently sequential)
