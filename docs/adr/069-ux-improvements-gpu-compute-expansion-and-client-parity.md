# ADR-069: UX Improvements, GPU Compute Expansion, and Client Parity

**Status:** Implemented
**Date:** 2026-04-10

## Context

TyphooN-Terminal has a mature native desktop client with 40 GPU compute
shaders, 150+ commands, 60+ drawing tools, and integrations with four
brokers. However, the web (WASM) client and CLI/TUI client lag far behind
in feature coverage. The web client renders only a close-price line chart,
polls every 5 seconds, has no indicators or watchlist, and lacks bracket/
trailing order support. The TUI renders ASCII-only charts with no order
entry beyond basic market orders.

Meanwhile, several GPU-acceleratable workloads still run on CPU, and the
chart rendering pipeline uses per-primitive CPU draw calls through egui
instead of instanced GPU rendering.

This ADR addresses three dimensions:

1. **UX improvements** — contextual command palette, recent commands,
   ruler tool, compact mode, position size preview, P&L heatmap, split
   sub-pane dividers
2. **GPU compute expansion** — new shaders for tick aggregation,
   batch screener, Renko construction, volume profile, rolling statistics,
   multi-symbol backtest; plus a GPU render pipeline for chart primitives
3. **Client equivalence** — protocol expansion with push messages, server-
   computed indicators, alerts, news; web client candlestick charts and
   trading parity; TUI braille charts and multi-broker support

## Decisions

### 1. Web Protocol Expansion

New `WebCmd` variants:

| Command | Fields | Purpose |
|---|---|---|
| `GetIndicators` | `symbol, timeframe, indicators: Vec<String>` | Request server-computed indicator values |
| `CreateAlert` | `symbol, condition, price, message` | Create price/indicator alert |
| `DeleteAlert` | `alert_id: String` | Remove alert |
| `ListAlerts` | — | List all active alerts |
| `GetNews` | `symbol: Option<String>` | Fetch news articles |

New `WebMsg` variants:

| Message | Fields | Purpose |
|---|---|---|
| `BarUpdate` | `symbol, timeframe, bar: BarData` | Real-time bar push (replaces polling) |
| `PositionUpdate` | `items: Vec<PositionSnapshot>` | Position change push |
| `AccountUpdate` | `AccountSnapshot` | Account change push |
| `IndicatorData` | `name, values: Vec<Option<f64>>` | Server-computed indicator results |
| `AlertTriggered` | `alert_id, symbol, message` | Alert notification push |
| `AlertList` | `items: Vec<AlertSnapshot>` | Active alerts |
| `NewsFeed` | `items: Vec<NewsItem>` | News articles |

Extended `PlaceOrder` fields:

| Field | Type | Purpose |
|---|---|---|
| `take_profit` | `Option<f64>` | Bracket order TP level |
| `stop_loss` | `Option<f64>` | Bracket order SL level |
| `trail_percent` | `Option<f64>` | Trailing stop percentage |
| `trail_offset` | `Option<f64>` | Trailing stop fixed offset |
| `risk_mode` | `Option<String>` | Risk mode (standard/fixed/dynamic/var) |
| `risk_pct` | `Option<f64>` | Risk percentage for server-side sizing |

### 2. Web Client Charting

Replace the close-price `egui_plot::Line` with proper OHLCV candlestick
rendering using `egui::Painter` primitives:

- Green body + wick for bullish bars, red for bearish
- Volume bars below the price chart
- Indicator overlays rendered as polylines from `IndicatorData` responses
- Zoom via scroll wheel, pan via drag
- Crosshair with price/time tooltip

### 3. Web Client Feature Additions

| Feature | Implementation |
|---|---|
| **Watchlist tab** | New `Tab::Watchlist`, sends `GetWatchlistQuotes`, renders grid with live bid/ask/change |
| **News tab** | New `Tab::News`, sends `GetNews`, renders article list |
| **Alerts tab** | New `Tab::Alerts`, create/delete/list with `AlertSnapshot` display |
| **Bracket orders** | TP/SL fields in order form, sent via extended `PlaceOrder` |
| **Trailing stops** | Trail % or offset field, conditional on order type |
| **Kraken routing** | `broker="kraken"` accepted for web/mobile order, cancel, and close commands; order types normalize to Kraken names per ADR-051 |
| **Risk modes** | Dropdown: Standard/Fixed/Dynamic/VaR with risk % input |
| **Push updates** | Handle `BarUpdate`/`PositionUpdate`/`AccountUpdate`, remove polling |
| **Quote ticks** | Render `QuoteTick` messages as live bid/ask in header |

### 4. UX Improvements (Native)

| Feature | Description |
|---|---|
| **Contextual palette** | Right-click context filters commands by area (chart → drawing/indicator, position → close/SL/TP) |
| **Recent commands** | Top 10 MRU commands shown at top of palette when filter is empty |
| **Ruler tool** | Temporary price/time/% distance overlay, disappears on mouse release |
| **Split sub-panes** | Draggable dividers between indicator sub-panes, double-click to collapse |
| **P&L heatmap** | Watchlist row background intensity scaled by daily % change |
| **Position size preview** | Shaded risk zone overlay showing $ at risk before order submission |
| **Compact mode** | Toggle between full analysis layout and minimal execution layout |
| **Server indicator relay** | Native app responds to `GetIndicators` by running GPU compute and sending results to web clients |

### 5. GPU Compute Expansion

New compute shaders:

| Shader | Dispatch | Description |
|---|---|---|
| **Tick Aggregation** | Parallel 256 | Aggregates raw ticks into OHLCV bars at multiple timeframes simultaneously |
| **Batch Screener** | Parallel 256 | Computes RSI/MACD/SMA across 500+ symbols in one dispatch (one thread per symbol) |
| **Renko Builder** | Sequential | GPU parallel scan for brick boundaries from OHLC data |
| **Volume Profile** | Parallel 256 | Histogram binning of price×volume into N price levels |
| **Rolling Statistics** | Parallel 256 | Rolling VaR, Sharpe, correlation windows (one thread per window position) |
| **Multi-Symbol Backtest** | Parallel 256×N | Tests strategy across N symbols × M parameter combos simultaneously |

### 6. GPU Render Pipeline

Replace per-primitive egui CPU draw calls with instanced GPU rendering:

| Pipeline | Description |
|---|---|
| **Candlestick Instanced** | Single draw call renders all visible candles as instanced quads (body) + line segments (wicks) |
| **Indicator Polyline** | GPU line strip shader — upload indicator value arrays, render as continuous polyline |
| **Heatmap Texture** | Compute values into GPU texture, render as single textured quad (correlation matrix, sector heatmap, volume profile) |
| **Zone Compositor** | Render session highlights, supply/demand zones, FVG to offscreen texture, composite once per frame |

### 7. TUI Client Upgrades

| Feature | Description |
|---|---|
| **Braille candlesticks** | Unicode braille (⠁⠃⠇⡇⣇⣧⣷⣿) + block elements (▀▁▂▃▄▅▆▇█) for high-resolution terminal charts |
| **Indicator lines** | Braille-dot polylines for SMA/EMA overlays in terminal |
| **Order form** | Interactive tui-input order entry: symbol, qty, side, type, price fields |
| **Multi-broker** | Add tastytrade and kraken API support to CLI (reuse engine broker modules) |
| **Alerts display** | Show active alerts and triggered notifications |

## Tests

**Total workspace test count: 836** (up from 813 in ADR-068).

- 216 typhoon-transpiler (unchanged)
- 497 engine (unchanged)
- 78 native (unchanged)
- 45 web-protocol (+23: push messages, alerts, news, indicators, subscribe,
  bracket/trailing order roundtrips, validation for risk_mode, alert_condition,
  indicator_name, alert_id, trailing_stop order type)

## Post-Implementation Audit (2026-04-10)

### Unwraps

| Crate | Production unwrap() | Production expect() | Notes |
|---|---|---|---|
| typhoon-transpiler/parser.rs | 0 | 0 | Rewritten: all `.next().unwrap()` → `next_or_err()` returning `CompileError::Internal` |
| typhoon-transpiler/ir.rs | 0 | 0 | Rewritten: all `.expect()` → `.ok_or_else(\|\| CompileError::Internal(...))` |
| engine | 0 | 0 | Clean |
| native | 0 | 0 | Clean |
| web-server | 0 | 0 | Clean |
| web-protocol | 0 | 0 | Clean |
| web | 0 | 0 | Clean |
| cli | 0 | 0 | Clean |

**Total production unwrap/expect: 0.** All 41 parser `.unwrap()` calls
rewritten to `next_or_err()` → `Result<_, CompileError::Internal>`.
All 9 IR `.expect()` calls rewritten to `.ok_or_else()`. Zero panics
possible from grammar/iterator mismatches across entire codebase.

### Security

- 0 unsafe blocks across entire codebase
- 0 SQL injection vectors (all queries use parameterized `params!` macro)
- 0 command injection (only `current_exe()` in NEW_WINDOW handler)
- 0 credential logging (keyring-only storage, no secrets in log messages)
- 0 path traversal (Darwin FTP validates with `validate_path_component()`)
- All WebCmd variants validated in web-server before relay
- `deny_unknown_fields` on all serde types
- Integer overflow checked in cache.rs with `checked_mul`/`checked_add`
- Write lock released before file I/O + compression in cache.rs export_backup
- Decompression failures logged instead of silently defaulted in cache.rs repair_bar_counts
- Client-side SYNCABLE_TABLES whitelist check on LAN sync table names (defense in depth)

### Performance

- VecDeque for web client alert_triggered (O(1) pop_front, was O(n) Vec::remove(0))
- All O(n²) loops resolved in prior audits (ADR-068)
- Remaining `.iter().find()` calls are on small collections (< 50 items)
- No lock contention issues — web-server drops Mutex guard before network I/O

### Shader Count

**Total WGSL shaders: 56** (50 from ADR-069 + 6 GPU parity from ADR-071)

Compute shaders (52): 33 indicators + 6 ADR-071 parity + 2 DARWIN analytics + 5 optimizer +
6 new (volume profile, batch screener, rolling stats, Renko, tick
aggregation, multi-symbol backtest).

Render shaders (4): candlestick instanced, polyline, heatmap texture,
zone compositor.

## Consequences

### Positive

- Web client becomes a viable mobile trading interface with real charts,
  indicators, bracket orders, and real-time push data — no more 5-second
  polling gaps.
- GPU render pipeline eliminates thousands of per-bar CPU draw calls,
  enabling smooth 60 FPS with 50K+ visible bars.
- New GPU shaders unlock batch screener (500+ symbols in one dispatch),
  real-time tick aggregation, and portfolio-level multi-symbol backtesting.
- TUI becomes usable for real trading over SSH with braille charts and
  interactive order entry.
- Protocol expansion is the single highest-leverage change — every web
  feature gap maps to a missing WebCmd/WebMsg variant.

### Trade-offs

- GPU render pipeline requires maintaining custom wgpu shaders alongside
  egui's built-in rendering. The two systems must coexist (GPU renders
  chart area, egui renders UI chrome).
- Server-computed indicators add load to the native app when web clients
  are connected. Rate limiting per client prevents abuse.
- Push-based data flow requires the native app to track which symbols
  each web client is viewing (subscription model vs. broadcast-all).
- Braille rendering in TUI depends on terminal Unicode support (most
  modern terminals support it, but some SSH clients may not).

## Related

- ADR-001 — Native GPU architecture (foundation)
- ADR-030 — GPU compute architecture (40 existing shaders)
- ADR-038 — GPU strategy optimizer
- ADR-053 — Web server TLS + WebSocket relay
- ADR-066 — Phone WASM trade tab
