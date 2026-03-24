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
- [x] 21 indicators: SMA, EMA, KAMA, WMA, HMA, Bollinger, Ichimoku, Parabolic SAR, ATR Projection, RSI, Fisher, MACD, Stochastic, ADX, CCI, Williams %R, OBV, Momentum, Better Volume, Volume, ATR
- [x] Sub-pane rendering for oscillators
- [x] NNFX core system (KAMA + Fisher + ATR Projection + Better Volume)

### Phase 4: UI Panels
- [x] Quake console (`~`) command palette with 50+ commands
- [x] Tab bar (Ctrl+N/W/Tab)
- [x] MTF grid (4-cell 2x2 layout)
- [x] Right panel: positions (DARWIN data), orders, risk, watchlist
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

## In Progress

### Phase 7: Broker Connection
- [ ] Alpaca broker connection (async tokio runtime)
- [ ] Live positions + orders from Alpaca WebSocket
- [ ] Order placement (market, limit, stop, bracket, trailing)
- [ ] Real-time bar updates via WebSocket streaming
- [ ] Bid/ask spread display

### Phase 8: tastytrade Integration
- [ ] tastytrade REST API client
- [ ] Symbol comparison (tastytrade vs Alpaca coverage)
- [ ] Options chain + Greeks

## Future

### Phase 9: Advanced Features
- [ ] More drawing tools (pitchfork, Elliott, Gann)
- [ ] Price alerts system
- [ ] Trade journal
- [ ] Pattern recognition (double top/bottom, H&S)
- [ ] Supply/demand zones

### Phase 10: Data Feeds
- [ ] News feed (Alpaca/Finnhub)
- [ ] Economic calendar
- [ ] SEC filings (EDGAR)
- [ ] Analyst ratings (Finnhub)
- [ ] FRED economic data

### Phase 11: Bookmap
- [ ] Order book depth heatmap (ADR-048)
- [ ] wgpu compute shader pipeline
- [ ] Level 2 WebSocket data

### Phase 12: Scripting
- [ ] MQL5 indicator compatibility layer
- [ ] PineScript-like DSL
- [ ] Hot-reload custom indicators
