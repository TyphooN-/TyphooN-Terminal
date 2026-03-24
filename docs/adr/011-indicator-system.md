# ADR-011: Indicator System (32+ Indicators)

**Status:** Implemented | **Date:** 2026-03-24

## Context
NNFX trading system requires specific indicators. Ehlers DSP indicators provide signal processing. Standard indicators for general analysis.

## Decision
All indicators computed in pure Rust on `&[f64]` slices. Pre-computed on load, cached in ChartState. Session-persistent toggles.

**NNFX Core:** SMA(200), KAMA(10,2,30), Fisher Transform(10), ATR Projection(14), Better Volume, Previous Candle Levels, Supply/Demand Zones, Fractals.

**Ehlers DSP (8):** Super Smoother, Decycler, Instantaneous Trendline, MAMA/FAMA, Even Better Sinewave, Cyber Cycle, CG Oscillator, Roofing Filter.

**Standard (14+):** EMA, WMA, HMA, Bollinger, Ichimoku, Parabolic SAR, RSI, MACD, Stochastic, ADX, CCI, Williams %R, OBV, Momentum, Volume.

**Patterns:** 7 Carney harmonics (Gartley, Butterfly, Bat, Crab, Shark, Cypher, 5-0), Pivot Points.

## Consequences
- Pro: < 15ms for all 32+ indicators on 10K bars
- Pro: Zero serialization (direct Rust computation)
- Pro: NNFX one-click preset via ~ console
