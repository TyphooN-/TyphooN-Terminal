# ADR-061: GPU/CPU Indicator Audit — Parity Verification

**Status:** Implemented | **Date:** 2026-03-28

## Context

Several GPU compute shaders were using different algorithms than their CPU fallbacks, producing incorrect results when the GPU path was active. An audit of all 28 GPU indicators was performed to verify parity.

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
- **GPU**: Simplified ratio thresholds (vol > 2x avg = climax, etc.)
- **CPU**: Now full 1:1 port of BetterVolume.mqh (buy/sell pressure estimation, lookback extremes, 2-bar analysis)
- **Fix**: CPU-only computation. GPU shader cannot do buy/sell estimation (requires open prices not in OHLC buffer, inherently sequential).

## Verified Matching (No Issues)

All other 25 GPU/CPU pairs produce identical results:
- SMA(200/100), EMA(21), KAMA(10,2,30), WMA(20), HMA(20)
- Bollinger(20,2), RSI(14), MACD(12,26,9), Stochastic(14,3,3)
- ATR(14), ADX(14), CCI(20), Williams%R(14), Momentum(10)
- Fisher(32), Ichimoku(9,26,52), Parabolic SAR
- Fractals (Bill Williams, 2-bar lookback — GPU returns price, CPU returns bool, conversion verified)
- All 8 Ehlers DSP indicators (SuperSmoother, Decycler, ITL, MAMA/FAMA, EBSW, CyberCycle, CG, Roofing)

## Consequences

- **Pro**: All indicators now produce correct, consistent results regardless of GPU availability
- **Pro**: GPU acceleration maintained for 25 of 28 indicators
- **Pro**: CPU fallback path verified functional for all indicators
- **Con**: BetterVolume is CPU-only (buy/sell estimation not parallelizable without open prices in VRAM)
- **Con**: S/D zone testing/merging remains CPU (inherently sequential)
