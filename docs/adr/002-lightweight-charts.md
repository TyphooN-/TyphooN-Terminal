# ADR-002: TradingView lightweight-charts for Charting

**Status:** Accepted
**Date:** 2026-03-15
**Context:** Need candlestick charting with draggable SL/TP lines, multi-pane layout, and real-time updates.

## Decision

Use TradingView's lightweight-charts (MIT-licensed open-source JavaScript library).

## Clarification

This is NOT the TradingView website/service. It's a standalone 170KB JavaScript canvas rendering library. No TradingView account or API required. All data comes from Alpaca. Runs 100% locally.

## Why

- Draggable price lines (essential for SL/TP)
- Per-bar coloring on histogram series (essential for Fisher/BetterVolume)
- Multiple chart instances for sub-panes (synced time scales)
- Real-time bar updates via `series.update()`
- Crosshair sync across panes
- MIT license, no restrictions, 170KB footprint

## Limitations

- No native per-bar line coloring (workaround: split into color segments)
- No native rectangles (workaround: baseline series for S/D zones)
- No true sub-windows (workaround: separate chart instances)
- TV watermark disabled via `attributionLogo: false`

See [INDICATOR_PORTING.md](../../INDICATOR_PORTING.md) for all porting lessons.
