# TyphooN Terminal — Keyboard Shortcuts

## Command Palette

| Key | Action |
|-----|--------|
| `~` (tilde/backtick) | Open Console (also: Tools → Console) |
| `Esc` | Close palette / cancel drawing mode / dismiss result card |
| `Enter` | Run the highlighted command |

## Navigation

| Key | Action |
|-----|--------|
| `←` / `→` | Bar-by-bar scroll |
| `Home` | Jump to start of data |
| `End` | Jump to latest bar |
| `Page Up` / `Page Down` | Half-screen scroll |
| `+` / `-` | Zoom in / out (horizontal) |
| Scroll wheel | Zoom (horizontal) |
| `Ctrl` + scroll | Zoom (vertical / price axis) |
| Click + drag | Pan chart |
| Double-click | Reset zoom and pan to auto-fit |
| `F5` | Refresh all analytics (marks indicators dirty) |

## Tabs & Timeframes

| Key | Action |
|-----|--------|
| `Ctrl+N` | New chart tab |
| `Ctrl+W` | Close current tab |
| `Ctrl+Tab` | Next tab |
| `Ctrl+Shift+Tab` | Previous tab |
| `Ctrl+1` … `Ctrl+9` | Jump to tab by number |
| `Alt+1` … `Alt+9` | Switch the active chart's timeframe (reload is deferred off the render thread) |

## Drawing & Chart

| Key | Action |
|-----|--------|
| `Alt+T` | Draw trend line (click 2 points) |
| `Alt+F` | Draw Fibonacci retracement |
| `Alt+H` | Draw horizontal line |
| `Alt+V` | Draw vertical line |
| `Alt+R` | Draw rectangle |
| `Alt+E` | Eraser mode |
| `Alt+C` | Cycle chart type (Candle → Heikin-Ashi → Line → OHLC → Renko) |
| `Alt+L` | Toggle log price scale |
| `Ctrl+Z` / `Ctrl+Shift+Z` | Undo / redo drawing |
| `Delete` / `Backspace` | Remove last drawing |
| Right-click | Context menu (drawing tools, chart type, copy price) |
| Click a drawing | Select it (drag body to move, drag square handles to resize) |
| `Esc` | Cancel drawing placement / deselect |

## Replay Mode

| Key | Action |
|-----|--------|
| `Space` | Play / pause replay |
| `←` / `→` | Step one bar back / forward (while paused) |
| `↑` / `↓` | Adjust replay speed |

## Menu Bar

| Menu | Key Items |
|------|-----------|
| File | Connect to Broker, Settings, Quit |
| View | MTF Grid, Chart Type, Indicators, Sub-Panes |
| Trading | Open Trade, Close All, SL/TP Lines, TradeCopy… |
| Tools | Backtest, Screener, Risk Calc |
| Research | News, Calendar, SEC, Fundamentals |
| Analysis | Correlation, Seasonals, Monte Carlo, Volume Profile |
| Help | Keyboard Shortcuts |

## Command Palette Commands

Open with `~` and type to fuzzy-search across registered research commands.
Per ADR-133, the command palette is intentionally research-only: drawing,
chart-type, chart-template, indicator-toggle, timeframe, SL/TP, screenshot, and
other graphical chart controls live in the chart toolbar/navbar/right-panel UI,
not in typed commands. The console additionally accepts research-surface
commands and aliases not listed in the registry (for example RESUME* AI resume
commands and ASKHERMES). Selected examples:

| Command | Action |
|---------|--------|
| `KRAKEN` | Kraken crypto exchange (connect, balance, trade) |
| `SETTINGS` | Application settings |
| `TRADECOPY` | Copy positions / mirror orders between broker accounts (opt-in) |
| `BACKTEST` | Run backtest on loaded bars |
| `OPTIMIZER` | Strategy parameter optimizer |
| `RISK_CALC` | Position sizing calculator |
| `VAR` | VaR multiplier estimator |
| `MARGIN` | Margin monitor |
| `SCREENER` | Symbol screener |
| `SYM` / `SYMBOLS` | Symbol Explorer (catalog browser) |
| `CORRELATION` | Correlation matrix |
| `SEASONALS` | Monthly return patterns |
| `MONTECARLO` | Monte Carlo VaR simulation |
| `STRESS_TEST` | Portfolio stress test |
| `RESEARCH_PACKET` | Research packet viewer |
| `REG_SHO` / `HALTS` | Regulatory outlier windows (ADR-120) |
| `BOOKMAP [SYM]` | Bookmap-style depth window |
| `CACHE_STATS` | Show cache statistics |
| `CLOSE_WINDOWS` | Close all floating windows |
| `HELP` | Keyboard shortcuts |
| `QUIT` | Exit (saves session) |
