# TyphooN Terminal — Roadmap

## Completed

### Phase 1: Foundation
- [x] eframe + egui + wgpu native window
- [x] OLED dark theme (#000000)
- [x] SQLite cache integration
- [x] Single chart viewport with candlestick rendering
- [x] Symbol/timeframe toolbar

### Phase 2: Chart Engine
- [x] 5 chart types: Candle, HeikinAshi, Line, OHLC Bars, Renko
- [x] Zoom (scroll + Ctrl+scroll), pan (drag), double-click reset
- [x] Price/time axis labels with smart date formatting
- [x] Crosshair with OHLCV + indicator values
- [x] Last-price line (dashed, color-coded)
- [x] SL/TP planning lines

### Phase 3: Indicators
- [x] 32+ indicators: SMA, EMA, KAMA, WMA, HMA, Bollinger, Ichimoku, Parabolic SAR, ATR Projection, RSI, Fisher(32), MACD, Stochastic, ADX, CCI, Williams %R, OBV, Momentum, Better Volume, Volume, ATR, Ehlers (8 DSP indicators)
- [x] Sub-pane rendering with MT5-matching histogram/line coloring
- [x] NNFX default preset (SMA200 + KAMA + Fisher + ATR Proj + BetterVol + PrevLevels + S/D Zones)

### Phase 4: UI Panels
- [x] Console (`~`) with 125+ commands
- [x] Tab bar with drag-and-drop reordering (Ctrl+N/W/Tab)
- [x] MTF grid (2×2 to 4×4, up to 16 charts)
- [x] Right panel: tabbed (Trade/Pos/Ord/WL/Risk), TradingView-style watchlist
- [x] Bottom panel: log + volume bars
- [x] 29 floating windows
- [x] Right-click context menu with drawing tools + chart type switcher
- [x] 7 drawing tools: HLine, TrendLine, Fibonacci, VLine, Rectangle, Ray, Channel
- [x] Session persistence (save/restore on quit/startup)
- [x] CSV export with file dialog

### Phase 5: DARWIN Analytics
- [x] XLSX import via rfd file dialog
- [x] Account overview: balance, P&L, win rate, profit factor, drawdown
- [x] Per-DARWIN VaR (95/99), Sharpe, Sortino, daily vol
- [x] Monthly returns, streak analysis, hourly P&L
- [x] Portfolio dashboard: combined equity, net P&L, max drawdown
- [x] Portfolio VaR with risk metrics
- [x] Correlation matrix with 0.95 warning threshold
- [x] Symbol exposure / overlap across DARWINs
- [x] Open positions from DARWIN data in right panel

### Phase 6: Engine Integration
- [x] Risk calculator (risk.rs: lot sizing, R:R)
- [x] Margin monitor (margin.rs: margin level, max safe lots, protect urgency)
- [x] Backtest engine (SMA Cross + NNFX strategies)
- [x] Optimizer (SMA Cross grid search, top N results)
- [x] Seasonals (monthly return patterns from bar data)
- [x] Volume Profile (POC, Value Area High/Low)
- [x] Monte Carlo VaR (from DARWIN daily returns)
- [x] Stress test (8 historical scenarios)
- [x] VaR multiplier (per-DARWIN corridor status)
- [x] Screener (cache symbol browser)

### Phase 7: Broker Connection
- [x] Alpaca broker connection (async tokio runtime)
- [x] Live positions + orders from Alpaca WebSocket
- [x] Order placement (market, limit, stop, bracket, trailing)
- [x] Real-time bar updates via WebSocket streaming
- [x] Bid/ask spread display

### Phase 8: Data & Analytics
- [x] Crypto backfill (Kraken — BTC daily from 2013, weekend gap-fill)
- [x] DARWIN signal vs quote comparison
- [x] MTF SMA (H1/H4/D1/W1 200SMA + W1/MN1 100SMA — Tomato + Magenta)
- [x] ATR Projection MTF (M15/H1/H4/D1/W1/MN1 horizontal levels)
- [x] Previous Candle Levels (H1/H4/D1/W1/MN1)
- [x] CLI/TUI (search, movers, fills)
- [x] Monthly Returns Heatmap (Darwinex-style grid per DARWIN)
- [x] Drawdown Analytics (combined + per-DARWIN dashboard, best/worst days)
- [x] Divergence Index (signal vs quote return divergence)
- [x] CAGR, Recovery Factor, Drawdown Duration
- [x] LAN Sync (export/import cache data between machines)
- [x] Storage Manager (view, delete, compact zstd-22 per symbol/source)
- [x] Multi-window support (NEW_WINDOW/POPOUT for multi-monitor)
- [x] Collapsible right panel sections
- [x] Sortable columns (SEC filings, insider trades tables)

## In Progress

### Phase 9: tastytrade Integration
- [x] tastytrade REST API client (auth only — session-based login)
- [ ] Market data via DXLink WebSocket
- [ ] Symbol comparison (tastytrade vs Alpaca coverage)
- [ ] Options chain + Greeks

## Future

### Phase 10: Advanced Features
- [ ] More drawing tools (pitchfork, Elliott, Gann)
- [x] Price alerts system (indicator-based: RSI, MACD, Fisher, Price conditions)
- [x] Trade journal (log trades with notes, ~ → JOURNAL)
- [ ] Pattern recognition (double top/bottom, H&S)
- [x] Supply/demand zones (auto-detected from impulse candles)

### Phase 11: Data Feeds
- [x] News feed (Finnhub)
- [ ] Economic calendar
- [x] SEC filings (EDGAR — full-text search, insider trades)
- [x] Analyst ratings (Finnhub consensus: buy/hold/sell + price targets)
- [x] FRED economic data (Fed Funds, CPI, GDP, Treasury yields, VIX, M2 Supply)

### Phase 12: Bookmap
- [ ] Order book depth heatmap (ADR-048)
- [ ] wgpu compute shader pipeline
- [ ] Level 2 WebSocket data

### Phase 13: Scripting
- [ ] MQL5 indicator compatibility layer
- [ ] PineScript-like DSL
- [ ] Hot-reload custom indicators
