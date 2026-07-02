# TyphooN Terminal — Design Philosophy

## Core Principles

### 1. Zero Overhead Data Path

Every byte from SQLite cache to GPU pixels traverses zero serialization layers. No JSON, no IPC, no JavaScript objects, no garbage collection. Rust types from storage to render.

### 2. Independent Risk Analytics

The terminal computes its own risk verification — VaR, CVaR, correlation, exposure, streaks, hourly P&L, drawdown — from cached bars and live broker positions, without relying on any broker's own dashboards. See `risk.rs`, `var.rs`, `margin.rs`, and the research surfaces in `research.rs`.

### 3. Trusted Data + Independent Corroboration

Market data currently comes from **Kraken + Alpaca** (trusted, corporate-action-adjusted where applicable) with Yahoo Chart as the independent corroborator. The design is broker-modular: L1/L2/L3 support must remain robust regardless of selected primary broker, with provider capabilities and entitlement limits modeled explicitly. tastytrade is a likely future restoration after the Alpaca/Kraken combover; Binance is a plausible later crypto venue. Equity bars from multiple providers are merged into one continuous, scale-validated series, and a bad trusted print is corrected against the corroborator rather than charted (ADR-111/112/113).

### 4. NNFX System Equivalence

The indicator system follows the No-Nonsense Forex (NNFX) methodology:
- KAMA(10,2,30) for trend direction
- Fisher Transform(32) for confirmation
- ATR Projection(14) for volatility bands
- Better Volume for volume analysis
- SMA(200) as baseline

All ported from MQL5 with computational equivalence to the originals.

### 5. Quake Console Interface

The `~` key opens a Quake-style dropdown command palette with fuzzy search across 260+ commands. Every panel, feature, and chart type is accessible via typed command.

### 6. Immediate Mode Rendering

egui's immediate mode paradigm means the entire UI is a function of state — no retained widget tree, no state synchronization bugs, no DOM diffing. The UI is redrawn every frame from the current `TyphooNApp` state.

### 7. Engine as Library

The `typhoon-engine` crate exports all broker, cache, risk, analytics, and backtest functionality as a Rust library. The active product surface is the native GUI. The former CLI/TUI consumer is archived on `deprecated/cli-tui` and no longer builds on `master`. Live depth profile overlays and richer per-order Bookmap views are available on focused symbols.

### 8. Risk Corridor Discipline

Risk panels enforce a configurable risk framework rather than ad-hoc sizing:
- VaR corridor with configurable upper/lower bounds
- Portfolio correlation limit
- Forward-looking TRIM for position sizing
- Margin and exposure monitoring

### 9. Session Continuity

Indicator toggles, symbol, MTF state, and window positions persist to `~/.config/typhoon-terminal/session.json`. The terminal resumes exactly where you left off.

### 10. Security

- No external JavaScript execution
- No WebView (no XSS, no CSP issues)
- SQLite parameterized queries only (no SQL injection)
- API credentials stored via OS-native keyring (libsecret/Keychain/CredentialManager)
