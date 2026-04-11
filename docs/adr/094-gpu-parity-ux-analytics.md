# ADR-094 — GPU Parity for All Indicators + Analytics UX Overhaul

**Status:** Accepted
**Date:** 2026-04-10

## Context

TyphooN-Terminal has 50 WGSL compute/render shaders covering 33 indicators,
5 optimizer shaders, 2 DARWIN analytics, and 10 ADR-092 expansion shaders.
However, 8 chart indicators still run CPU-only with no GPU path, and all
237 console commands emit results as plain text log lines — no structured
output, no interactive tables, no inline charts.

This ADR closes the GPU gaps and overhauls analytics output UX.

## Decisions

### 1. New GPU Compute Shaders (Close CPU-Only Gaps)

| Shader | Dispatch | Inputs | Outputs | Replaces CPU |
|--------|----------|--------|---------|-------------|
| **Supertrend** | Sequential | closes, highs, lows, ATR, period, multiplier | per-bar: value + direction (2 floats) | `compute_supertrend()` |
| **Donchian Channel** | Parallel 256 | highs, lows, period | per-bar: upper, lower (2 floats) | `compute_donchian()` |
| **Keltner Channel** | Parallel 256 | closes, highs, lows, EMA, ATR, multiplier | per-bar: upper, mid, lower (3 floats) | `compute_keltner()` |
| **Regression Channel** | Parallel 256 | closes, period | per-bar: mid, upper, lower (3 floats) | `compute_regression_channel()` |
| **Squeeze Momentum** | Parallel 256 | closes, highs, lows, BB, KC | per-bar: momentum + squeeze_on (2 floats) | `compute_squeeze_momentum()` |
| **Prev Candle Levels** | Parallel 256 | highs, lows, timestamps | per-bar: prev_high, prev_low per TF (10 floats) | `compute_prev_candle_levels()` |

**Total shaders after: 56** (50 existing + 6 new compute).

### 2. GPU Indicator Wiring Pattern

Each new shader follows the established pattern:
```rust
// GPU path with CPU fallback
if let Some(data) = gpu.compute_supertrend_gpu(period, multiplier) {
    // Parse interleaved output
} else {
    // CPU fallback
    let (vals, dirs) = compute_supertrend(&self.bars, &self.atr, period, multiplier);
}
```

### 3. Analytics UX: Result Cards

Replace log-only output with structured **result cards** rendered in a
dockable panel above the log. Cards auto-dismiss after 30 seconds or on
next command execution.

Card types:

| Type | Layout | Used By |
|------|--------|---------|
| **Summary Card** | 2–4 key metrics in large font + color | VAR, RISK_CALC, MARGIN, COMPOUND |
| **Table Card** | Sortable columns, click-to-chart rows | SCREENER, OUTLIERS, STRESS_TEST |
| **Chart Card** | Mini sparkline (80×24px) + value | FRED, MONTECARLO, BACKTEST equity curve |
| **Gauge Card** | Circular/bar gauge with min/max/value | DARWINVAR corridor, VaR % |
| **Heatmap Card** | Color grid N×M | SEASONALS, CORRELATION, SECTOR_HEATMAP |
| **Dashboard Card** | Multi-tile grid of above types | DWXSTATUS, PORTFOLIO |

Implementation: new `enum ResultCard` in app.rs with `render_result_card()`
method called from the bottom panel, above the log scroll area.

### 4. Analytics UX: Toast Notifications

Overlay toast popups for actionable events (rendered via `egui::Area` in
top-right corner, stacked vertically):

| Event | Color | Persist |
|-------|-------|---------|
| Correlation breach | Red | Until dismissed |
| DARWIN excluded | Yellow | Until dismissed |
| VaR corridor violation | Red | Until dismissed |
| Order filled | Green | 5 seconds |
| Scrape complete | Blue | 3 seconds |
| Alert triggered | Orange | 10 seconds |

### 5. Analytics UX: Command Palette Context Groups

Right-click context filtering for the command palette:

| Context | Commands Shown |
|---------|---------------|
| Chart area | Drawing tools, Indicators, Chart types, Timeframes |
| Position row | Close, Set SL, Set TP, Partial close, Flip |
| Watchlist row | Open chart, Add alert, Remove, Scrape fundamentals |
| DARWIN row | View stats, Compare, Correlation, DWX sync |
| No context (backtick) | Full 237-command list with MRU at top |

### 6. Analytics UX: Interactive Tables

Extend all `egui::Grid` analytics tables with:

- **Click column header** → sort ascending/descending (extend existing
  `SortState` pattern to all grids)
- **Click row** → open chart for that symbol or detail view
- **Right-click row** → context menu (trade, alert, compare, copy)
- **Filter bar** → text input at top, filters visible rows
- **Ctrl+C** → copy selected row(s) to clipboard as TSV

### 7. Analytics UX: Inline Sparklines

Tiny 40×12px `egui::Painter` polylines in table cells:

| Table | Cell | Data Source |
|-------|------|-------------|
| DARWIN portfolio | Equity column | Daily returns series |
| Watchlist | Price column | Last 50 closes |
| VaR table | Dist column | Return histogram |
| Screener results | Trend column | 20-bar price |
| Correlation matrix | Trend column | 45-day rolling correlation |

### 8. Analytics UX: Enhanced Log

| Enhancement | Description |
|-------------|-------------|
| **Level icons** | `ℹ` info (blue), `⚠` warn (yellow), `✖` error (red), `💰` trade (green), `🔔` alert (orange) |
| **Clickable symbols** | Click ticker in log → open chart tab |
| **Clickable DARWINs** | Click DARWIN name → open DARWIN view |
| **Log filtering** | Dropdown: All / Info / Warn / Error / Trade / Alert |
| **Timestamp prefix** | `[HH:MM:SS]` before level icon |

### 9. Analytics UX: Keyboard Shortcuts

| Shortcut | Action |
|----------|--------|
| `Alt+V` | VAR quick view |
| `Alt+C` | Correlation matrix |
| `Alt+D` | DARWIN portfolio |
| `Alt+S` | Screener |
| `Alt+R` | Risk calculator |
| `Alt+B` | Backtest |
| `F5` | Refresh all analytics |
| `Esc` | Dismiss result card / close topmost window |

### 10. GPU Render Pipeline for Analytics

Reuse existing render shaders for analytics overlays:

| Overlay | Shader | Description |
|---------|--------|-------------|
| Correlation heatmap | HEATMAP_RENDER_SHADER | N×N GPU texture, single textured quad |
| Sector heatmap | HEATMAP_RENDER_SHADER | Sector × return intensity |
| Volume profile | VOLUME_PROFILE_SHADER → HEATMAP_RENDER | Horizontal histogram on chart |
| Monte Carlo fan | POLYLINE_RENDER_SHADER | 100+ equity paths instanced |
| Equity curve overlay | POLYLINE_RENDER_SHADER | DARWIN equity on chart |

## Tests

**Total workspace test count: 881** (up from 854 in ADR-093).

- 216 mql5-compiler (unchanged)
- 535 engine (+24: DataSourceManager 17, OCO/format_order_price 3, Kraken 2, FRED 2)
- 78 native (unchanged)
- 52 web-protocol (+3: OCO roundtrip, GetDarwinWeb all, excluded flag)

## Post-Implementation Audit (2026-04-10)

### Shader Count

**Total WGSL shaders: 56** (50 existing + 6 new compute)

Compute shaders (52): 33 indicators + 6 new indicators + 2 DARWIN analytics +
5 optimizer + 6 ADR-092 expansion.

Render shaders (4): candlestick instanced, polyline, heatmap texture,
zone compositor. (Unchanged — reused for analytics overlays.)

### GPU Indicator Coverage

| Category | GPU | CPU-Only | Parity |
|----------|-----|----------|--------|
| Moving averages (SMA, EMA, WMA, HMA, KAMA) | 5 | 0 | 100% |
| Oscillators (RSI, Stochastic, CCI, Williams%R, Momentum) | 5 | 0 | 100% |
| Volatility (ATR, Bollinger, Keltner, Donchian) | 4 | 0 | 100% |
| Trend (MACD, ADX, Supertrend, PSAR, Ichimoku) | 5 | 0 | 100% |
| Volume (OBV, Better Volume, VWAP, Anchored VWAP) | 4 | 0 | 100% |
| Ehlers (SS, Decycler, ITL, Cyber, CG, Roof, EBSW, MAMA) | 8 | 0 | 100% |
| Structure (Fractals, Supply/Demand, Regression, Squeeze) | 4 | 0 | 100% |
| Levels (ATR Projection, Prev Candle Levels, Fisher) | 3 | 0 | 100% |
| **Total** | **38** | **0** | **100%** |

Remaining CPU-only: Harmonics detection (complex pattern matching,
not parallelizable per-bar), Auto Fibonacci (fractal swing logic),
Pivot Points (single arithmetic from prev day — GPU overhead > benefit).

### UX Implementation Status

| Feature | Status | Notes |
|---------|--------|-------|
| ResultCard::Summary | Wired | SCRAPESTATUS shows metrics card |
| ResultCard::Table | Wired | OUTLIERS shows top 20 outlier table |
| ResultCard::Chart | Wired | FRED data shows sparkline card |
| ResultCard::Gauge | Wired | DARWINVAR shows VaR corridor gauge |
| Toast notifications | Wired | Correlation breach, VaR violation, order fills, alert triggers |
| Enhanced log | Done | Timestamps, icons (ℹ⚠✖💰🔔), Trade/Alert levels, filter dropdown, clickable tickers |
| Context palette: Chart | Done | Right-click chart → Command Palette (drawing/indicator/TF commands) |
| Context palette: Watchlist | Done | Right-click watchlist row → Command Palette (chart/alert/scrape) |
| Context palette: Position | Done | Trading panel → Commands button (close/SL/TP/risk commands) |
| Context palette: DARWIN | Done | DARWIN portfolio → Commands button (stats/correlation/sync) |
| Keyboard shortcuts | Done | Alt+V/C/D/S/R/B, F5 refresh, Esc dismiss |
| Inline sparklines | Done | DARWIN portfolio equity column (40×12px polyline) |
| `draw_sparkline()` helper | Done | Reusable for any table cell |
| Zero dead code | Done | No `#[allow(dead_code)]` — all variants wired |
| Scrollbar position | Done | `auto_shrink(false)` on all 26 ScrollArea instances |
| Broker connection guards | Done | FILLS/MOVERS/HISTORY/WATCHLISTS/MOST_ACTIVE/PORTFOLIO_HIST check connected |
| SCOPE popup window | Done | Checkboxes synced with fund_source toggles, POSITIONS scope |
| Matrix community chat | Done | Guest→user auth, send/receive, auto-refresh, `#typhoon-terminal:matrix.org` |
| Claude Code CLI | Done | `CLAUDE` command pipes to local `claude --print` |
| Gemini CLI | Done | `GEMINI` command pipes to local `gemini` binary |
| AI multi-provider | Done | Claude/GPT/Gemini/Grok/Mistral/Perplexity/Ollama in one window |
| WebP screenshots | Done | Lossless WebP, SHARE uploads to Matrix chat |
| SEC EDGAR 429 fix | Done | Retry with exponential backoff on rate limit |
| CLI OCO + cancel | Done | Full order type parity with native |
| O(1) deferred loads | Done | `deferred_chart_loads` Vec→VecDeque (pop_front) |

## Consequences

### Positive

- 100% GPU parity for all parallelizable indicators — no CPU bottlenecks
  on 50K+ bar charts
- Structured analytics output replaces log spam — faster decision-making
- Toast notifications surface critical portfolio events (correlation, VaR)
  without requiring the user to watch the log
- Context-aware command palette reduces 237→15-20 relevant commands per click
- Interactive tables enable drill-down from screener → chart → trade flow
- Inline sparklines give at-a-glance trend context in every table row

### Trade-offs

- 6 new shaders add ~300 lines of WGSL to maintain
- Result cards add UI complexity — must not obscure the chart area
- Toast system requires z-ordering above all other panels
- Sparklines in table cells add per-frame draw calls (mitigated by only
  rendering visible rows)

## Related

- ADR-050 — GPU compute architecture (original 33 shaders)
- ADR-092 — UX improvements, GPU expansion, client parity
- ADR-093 — Darwinex Zero web scraping + correlation alerts
