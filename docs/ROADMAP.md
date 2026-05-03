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
- [x] 46+ chart indicators: SMA, EMA, KAMA, WMA, HMA, Bollinger, Ichimoku, Parabolic SAR, ATR Projection, RSI, Fisher(32), MACD, Stochastic, ADX, CCI, Williams %R, OBV, Momentum, Better Volume, Volume, ATR, Ehlers (8 DSP indicators), CMO, QStick, Disparity, BOP, StdDev (ADR-200)
- [x] Sub-pane rendering with MT5-matching histogram/line coloring
- [x] NNFX default preset (SMA200 + KAMA + Fisher + ATR Proj + BetterVol + PrevLevels + S/D Zones)

### Phase 4: UI Panels
- [x] Console (`~`) with 205+ commands
- [x] Tab bar with drag-and-drop reordering (Ctrl+N/W/Tab)
- [x] MTF grid (2×2 to 4×4, up to 16 charts)
- [x] Right panel: tabbed (Trade/Pos/Ord/WL/Risk), TradingView-style watchlist
- [x] Bottom panel: log + volume bars
- [x] 54+ floating windows
- [x] Right-click context menu with drawing tools + chart type switcher
- [x] 89 drawing tools (lines, channels, Fibonacci, shapes, Gann, Elliott, measurement, patterns, annotations, position, cycles, projection, curves)
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
- [x] Backtest engine (5 strategies: SMA Cross, NNFX, KAMA Cross, Fisher Cross, RSI Mean-Rev)
- [x] Optimizer (SMA Cross grid search, top N results)
- [x] Walk-forward optimizer (70/30 in-sample/out-of-sample, 5 strategies)
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
- [x] Crypto backfill (CryptoCompare — BTC from 2010, 2000 bars/request; Kraken retained for weekend gap-fill)
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
- [x] CryptoCompare deep history (BTC from 2010, replaces Kraken as primary backfill)
- [x] Weekend crypto adaptive polling (60s M1, 2.5min M15, 5min H1+) with magenta candles
- [x] Chart right margin (5 bars, MT5 chart shift style)
- [x] Unusual Volume Scanner
- [x] Multi-signal Anomaly Scanner (VaR + EV + ATR + SEC with tradability indicators)
- [x] MTF Grid tab visibility checkboxes
- [x] Storage Manager pagination
- [x] Data hierarchy: MT5 > Broker > CryptoCompare > Kraken

### Phase 9: tastytrade Integration
- [x] tastytrade REST API client (session-based login, balances, positions, orders)
- [x] Market data via DXLink WebSocket (historical bars: SETUP→AUTH→FEED protocol)
- [x] Option chains + Greeks (nested expiration/strike, IV rank/percentile via market metrics)
- [x] Quote snapshots + market metrics (bid/ask, IV rank, IV percentile, beta)
- [x] 5-source bar priority: MT5 → Alpaca → tastytrade → CryptoCompare → Kraken

### Phase 10: Advanced Features
- [x] More drawing tools (pitchfork, Elliott, Gann — all implemented, 89 total)
- [x] Price alerts system (indicator-based: RSI, MACD, Fisher, Price conditions)
- [x] Trade journal (log trades with notes, ~ → JOURNAL)
- [x] Supply/demand zones (auto-detected from impulse candles, GPU + CPU paths)
- [x] Harmonic patterns (Gartley, Butterfly, Bat, Crab, Shark, Cypher, 5-0, Alt Bat, Deep Crab, Three Drives)
- [x] Position visibility toggles per broker (DARWIN/Alpaca/tastytrade)
- [x] POSITION_CHARTS command (open W1 tabs for all open positions)
- [x] Backfill candle coloring (magenta for non-primary data sources)
- [x] Session save on window close (on_exit)

### Phase 11: Data Feeds
- [x] News feed (Finnhub)
- [x] Economic calendar (Finnhub — FOMC, NFP, CPI, PMI with impact ratings)
- [x] SEC filings (EDGAR — full-text search, insider trades)
- [x] Analyst ratings (Finnhub consensus: buy/hold/sell + price targets)
- [x] FRED economic data (Fed Funds, CPI, GDP, Treasury yields, VIX, M2 Supply)

### Phase 12: MQL5/PineScript Compiler
- [x] MQL5 parser (pest grammar → AST, core MQL5 syntax, 216 tests)
- [x] WASM backend (CPU execution via wasmtime)
- [x] WGSL backend (GPU execution via wgpu compute shaders)
- [x] PineScript v5 parser (indicator, input.*, ta.*, plot, math.*)
- [x] 8 transpiler frontends/backends: MQL4, PineScript, ThinkScript, EasyLanguage, AFL, ACSIL, cAlgo, NinjaScript, ProBuilder
- [ ] Hot-reload custom indicators from file
- [ ] Indicator marketplace / import UI

### Phase 13: Kraken Broker
- [x] Public OHLCV ingest (gap-fill source)
- [x] HMAC-SHA512 signed REST trading (ADR-072)
- [x] Full Spot REST AddOrder parameters: stop/take-profit/trailing variants, price2, displayvol iceberg, settle-position, margin/reduce-only, flags, TIF, client IDs, STP, validate-only, conditional close
- [x] Batch orders, order amend/edit, batch cancel, cancel-all, dead-man cancel
- [x] LAN web/mobile order, cancel, and close routing for Kraken
- [x] Position summaries unified into PositionInfo shape (ADR-201)
- [x] Display-pair normalization (XBTUSD → BTCUSD)

### Phase 14: LAN Sync v2
- [x] TLS-encrypted (wss://) WebSocket sync, ephemeral self-signed certs (ADR-065)
- [x] PBKDF2 passphrase auth, constant-time HMAC-SHA256
- [x] 14 remote commands (SEC_SCRAPE, DARWIN_IMPORT, FETCH_BARS, etc.) — ADR-066
- [x] Bandwidth-tuned sync, full data + KV cache (ADR-079)

### Phase 15: Web LAN Client
- [x] WASM client (eframe/glow), built via trunk (ADR-073)
- [x] HTTPS + WebSocket relay (axum, axum-server)
- [x] Read-only chart, watchlist, positions/orders display
- [x] PBKDF2 passphrase auth shared with native LAN sync

### Phase 16: Fundamentals & Research
- [x] Fundamentals engine across 21 sources (Alpaca, Finnhub, FMP, Alpha Vantage, FRED, SEC EDGAR, Yahoo, etc.) — ADR-054
- [x] Fundamentals research packet (markdown bundle for AI agents)
- [x] News earnings dividends pipeline (ADR-012, ADR-107)
- [x] Notification system: Discord webhook, Pushover, ntfy.sh, Matrix (ADR-074)

### Phase 17: AI Sessions
- [x] Persistent AI sessions: Claude Code, Gemini CLI, Codex CLI, generic AI Chat (ADR-157)
- [x] Cross-client AI response cache, dedup across LAN clients (ADR-162)
- [x] Slash commands: RESUMECLAUDE / RESUMEGEMINI / RESUMECODEX / RESUMEAI
- [x] Ask Codex reasoning effort control

### Phase 18: TA-Lib + Godel Parity
- [x] ~375 TA-Lib primitives (indicators + candlestick patterns) across 75+ rounds
- [x] Godel-Terminal-documented features (options chain, expirations calendar, earnings whispers, institutional ownership, insider transactions)
- [x] Research-packet pipeline as the AI-agent-readable surface (ADR-188)
- [x] Chart-parity reopened for chartable oscillator/stat bundles (ADR-200)
- [ ] Chart-overlay candlestick pattern marks (deferred — ADR-188)

### Phase 19: MT5 EA Trading-Flow Alignment
- [x] One net position per symbol across Alpaca / tastytrade / Kraken (ADR-201)
- [x] Partial close + close-all on every broker
- [x] Cancel-pending-exit-orders-before-close (no more `insufficient qty` rejects)
- [x] Display-symbol normalization to EA's symbol table

### Phase 20: Performance & Compile-Speed
- [x] native/app.rs split into submodules: ai, settings, storage, sync_status, tool_windows, strategy_windows (ADR-202)
- [x] Alpaca sync autotuning by data tier (ADR-203)
- [x] Header-driven rate-limit pacing, cache-depth-aware window sizing
- [x] No-data symbol skip set
- [x] Dependency audit + RustSec advisory closure (ADR-204)

### Phase 21: Headless LAN & Observability
- [x] CLI `--lan-server` / `--lan-client` headless mode reusing GUI keyring/KV passphrase (ADR-206)
- [x] `--lan-passphrase` / `TYPHOON_LAN_PASSPHRASE` bootstrap for fresh caches (ADR-206)
- [x] Docker, Docker Compose, Kubernetes (envsubst + hostPath PV), Terraform module, Ansible role (ADR-206)
- [x] CLI `--metrics-port` Prometheus text endpoint with cache size, row counts, per-series bar counts, liveness, uptime (ADR-209)
- [x] Compose `observability` profile + Ansible/Kubernetes/Terraform Prometheus + Grafana with provisioned LAN dashboard (ADR-209)
- [x] Compose `kafka` profile + Ansible/Kubernetes/Terraform single-node Apache Kafka KRaft broker (ADR-209)

## Future

### Phase 13: Bookmap
- [ ] Order book depth heatmap (ADR-048)
- [ ] wgpu compute shader pipeline
- [ ] Level 2 WebSocket data

### Phase 14: Pattern Recognition
- [ ] Classic chart patterns (double top/bottom, H&S, wedge, triangle)
- [ ] Auto-detection with confirmation signals
