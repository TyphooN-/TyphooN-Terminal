# TyphooN Terminal — Design Philosophy

## Core Principles

### 1. Zero Overhead Data Path

Every byte from SQLite cache to GPU pixels traverses zero serialization layers. No JSON, no IPC, no JavaScript objects, no garbage collection. Rust types from storage to render.

### 2. DARWIN-First Analytics

The terminal is built around Darwinex DARWIN accounts. The analytics engine (darwin.rs, 5,400+ lines, 70+ functions) provides independent risk verification — VaR, correlation, exposure, streaks, hourly P&L — without relying on Darwinex's own dashboards.

### 3. MT5 as View-Only Data Source

MT5 provides bar data via the BarCacheWriter EA → SQLite cache pipeline. Trade management stays in MT5. The terminal consumes MT5 data but does not manage MT5 instances. DARWIN analytics come from XLSX trade history imports.

### 4. NNFX System Parity

The indicator system matches the No-Nonsense Forex (NNFX) methodology:
- KAMA(10,2,30) for trend direction
- Fisher Transform(32) for confirmation
- ATR Projection(14) for volatility bands
- Better Volume for volume analysis
- SMA(200) as baseline

All ported from MQL5 with exact computational parity.

### 5. Quake Console Interface

The `~` key opens a Quake-style dropdown command palette with fuzzy search across 50+ commands. Every panel, feature, and chart type is accessible via typed command.

### 6. Immediate Mode Rendering

egui's immediate mode paradigm means the entire UI is a function of state — no retained widget tree, no state synchronization bugs, no DOM diffing. The UI is redrawn every frame from the current `TyphooNApp` state.

### 7. Engine as Library

The `typhoon-engine` crate exports all broker, cache, risk, analytics, and backtest functionality as a Rust library. The native GUI, CLI/TUI, and any future interface consume the same engine with zero duplication.

### 8. Darwinex VaR Corridor Compliance

All risk panels are designed around Darwinex's rules:
- VaR corridor: 3.25% – 6.5%
- Correlation limit: 0.95 / 45d
- 100% margin accounts
- Forward-looking TRIM for position sizing

### 9. Session Continuity

Indicator toggles, symbol, MTF state, and window positions persist to `~/.config/typhoon-terminal/session.json`. The terminal resumes exactly where you left off.

### 10. Security

- No external JavaScript execution
- No WebView (no XSS, no CSP issues)
- SQLite parameterized queries only (no SQL injection)
- API credentials stored via AES-256-GCM (keychain module)
- XLSX parsing via calamine (pure Rust, no shell execution)
