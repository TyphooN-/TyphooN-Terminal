# ADR-002: Chart Engine

**Status:** Implemented
**Date:** 2026-03-24

## Context

A trading terminal's primary interface is the price chart. It must render thousands of bars at 60 fps with interactive zoom, pan, and crosshair tracking. Off-the-shelf plotting libraries lack candlestick primitives and trading-specific interaction models.

## Decision

Implement a custom chart engine using egui's Painter API for direct shape rendering. Support 5 chart types: Candlestick, Heikin-Ashi, Line, OHLC bars, and Renko. Zoom is mouse-wheel driven (scaling visible bar count), pan is click-drag on the time axis, and crosshair snaps to the nearest bar with OHLC/volume tooltip. Price and time axes auto-scale. Use egui_plot for separate analytics panes (indicator sub-charts, equity curves) where interactive plotting is needed but candlestick rendering is not.

## Consequences

- Custom Painter rendering gives full control over candle geometry, wick thickness, and color schemes
- Heikin-Ashi and Renko are computed from raw OHLC data at render time; no separate data pipeline
- Zoom/pan state is per-chart, enabling independent navigation across multi-tab and MTF grid views
- Crosshair with price/time labels provides precise reading without cluttering the chart
- egui_plot handles indicator sub-panes with built-in axis linking and legend toggling
- Trade-off: custom renderer requires manual hit-testing for interactive elements (drawing tools, SL/TP lines)
