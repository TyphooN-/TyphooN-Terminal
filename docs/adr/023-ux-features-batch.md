# ADR-023: UX Feature Batch — GUI Menu, Tabs, Drawing Tools, Trading Improvements

**Status:** Implemented
**Date:** 2026-03-16

## Context

Multiple UX features were added to close competitive gaps with TradingView, MT5, and Godel Terminal. These are grouped into one ADR as they share the same motivation: making the terminal usable for mouse-driven traders who don't know keyboard shortcuts.

## Features

### GUI Menu Bar
- 6 dropdown menus: File, View, Trading, Tools, Research, Analysis
- 50+ menu entries routing to existing functions
- Keyboard shortcuts shown in labels for learning
- No new functionality — pure discoverability layer

### Draggable Tab Reordering
- HTML5 drag-and-drop on tab elements
- Green drop indicator (left/right border)
- Tab order persists in session state
- Feature no competitor has (not MT5, Godel, cTrader, NinjaTrader)

### TradingView-Style Drawing Tools
- **Ray** (E key): trend line extending to right edge
- **Ruler** (J key): measure price distance + % change between two points
- Total: 7 drawing tools (trendline, fibonacci, horizontal, rectangle, channel, ray, ruler)
- All accessible via keyboard, right-click context menu, and GUI menu

### Improved SL/TP Line Dragging
- Single click near line starts drag (was double-click)
- Hit tolerance increased 8px → 14px
- Works on MTF grid cells via `getActiveCandleSeries()`
- Live risk tooltip follows cursor during drag: direction, R:R, SL/TP distances
- Dashboard risk calculator panel permanently shows BUY/SELL, R:R, risk %

### MTF Grid Trading
- Single click grid cell to select (green outline)
- SL/TP lines create on active cell's candleSeries
- Buy/Sell Lines use active cell's data
- First cell auto-selected on grid open

### Custom Timeframes + Renko
- 2H, 3H, 6H, 2D, 3D via `aggregateBars()` from base timeframe
- Renko bars: ATR(14)-based brick size, chart type selector

### Pattern Recognition + Sentiment + Volatility Surface
- PATTERNS: auto-detect double top/bottom, head & shoulders with chart markers
- SENTIMENT: keyword-based bullish/bearish scoring from news
- VOLSURF: options IV heatmap by strike × expiry (3 monthly expirations)

### Other
- Heikin-Ashi candlestick chart type
- Risk/reward overlay (green/red zones when SL/TP set)
- Trade journal (persistent, Ctrl+K → JOURNAL)
- Position sizing calculator (Ctrl+K → CALC)
- Chart annotations (markers on bars)
- Regime detection (ADX-based trending/ranging/choppy)
- Multi-symbol alert dashboard (ALERTBOARD)
- AI trade review ("Review My Trades" button)
- Bracket order UI (BRACKET)
- Portfolio heat map (HEATMAP)

## Security

All features use createElement + textContent (zero innerHTML). No new Tauri commands needed for most features (pure frontend). Menu bar routes to existing validated functions.

## Consequences

- **Pro**: Terminal now fully usable without knowing any keyboard shortcuts
- **Pro**: TradingView-style drawing tools close the biggest UX gap
- **Pro**: Draggable tabs is a unique feature no competitor offers
- **Pro**: MTF grid trading enables analysis-to-execution on any timeframe
- **Con**: JS bundle grew from 330KB to 373KB (still under 110KB gzipped)
