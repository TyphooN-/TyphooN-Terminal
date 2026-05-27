# ADR-042: DARWIN Trade Overlay — Chart Arrows + Position Lines

**Status:** Implemented | **Date:** 2026-03-28

## Context

MT5 displays buy/sell arrows and position SL/TP lines directly on the chart. TyphooN Terminal had DARWIN position data in the right panel but no chart visualization.

## Decision

### Trade Markers (Arrows)
- Buy arrows (green, pointing up below price) and sell arrows (red, pointing down above price) rendered at deal entry/exit timestamps
- Deal timestamps parsed from MQL5 format ("YYYY.MM.DD HH:MM:SS") → epoch ms → binary search to find corresponding bar index
- Data sourced from `AccountDetailCache.closed_positions` (entries + exits) and `recent_deals`

### Position Lines
- Open position entry prices shown as dashed horizontal lines (blue for buy, orange for sell)
- Labels show direction + aggregated lot size (e.g., "BUY 2.40")
- Data sourced from `AccountDetailCache.open_positions`

### Aggregation (Critical for Darwinex)
Darwinex limits lot sizes, resulting in many small orders at the same price. Same-price entries at the same bar are aggregated into a single marker showing combined volume and count. Position lines at the same price are also merged.

Aggregation key: `(bar_index, is_buy, price_rounded_to_5_decimals)`

### Symbol Matching
Chart symbol is stripped to bare symbol (removing prefixes like `mt5:`, `cryptocompare:` and timeframe suffixes) for matching against DARWIN deal/position symbols.

### Positions Panel Filtering
Right panel Positions section shows current chart symbol's positions in full color first, then other symbols dimmed below.

## Implementation

`TradeOverlay` struct passed to `draw_chart()`:
- `markers: Vec<TradeMarker>` — buy/sell arrows with bar_idx, price, aggregated volume
- `position_lines: Vec<PositionLine>` — entry/SL/TP lines with price, volume, type

Built per-chart via `build_trade_overlay()` which scans all DARWIN account details for matching symbol.

## Consequences

- **Pro**: Visual trade history on chart matching MT5 style
- **Pro**: Aggregation prevents chart clutter from Darwinex lot-splitting
- **Pro**: Works for all DARWIN accounts simultaneously
- **Con**: Limited to recent deals (background cache fetches last 20 per DARWIN)
- **Future**: Expand to Alpaca/tastytrade trade history
