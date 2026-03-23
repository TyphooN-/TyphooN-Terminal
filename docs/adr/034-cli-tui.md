# ADR-034: CLI / TUI Terminal Interface

**Status:** Implemented (Phase 1 — Trading Core)
**Date:** 2026-03-19, updated 2026-03-23

## Context

The GUI terminal (Tauri + WebView) requires a display server. For VPS algorithmic trading, SSH monitoring, and headless operation, a text-based terminal interface is needed.

## Decision

Built a standalone CLI binary (`typhoon`) using `ratatui` + `crossterm` in Rust. The CLI shares:
- Encrypted credential storage (AES-256-GCM SQLite) with the GUI
- Same Alpaca REST API client (reimplemented without Tauri dependencies)
- Same MT5 CSV import format and account registry
- Same timeframe resolution (M1-MN1, custom aggregation)

## Architecture

```
┌─────────────────────────────────────────┐
│          TyphooN Terminal CLI           │
│  ┌─────────┐ ┌──────┐ ┌─────────────┐  │
│  │ ratatui │ │broker│ │   creds     │  │
│  │  (TUI)  │ │ .rs  │ │(AES decrypt)│  │
│  └─────────┘ └──────┘ └─────────────┘  │
│       │           │           │         │
│       └───────────┼───────────┘         │
│                   │                     │
└───────────────────┼─────────────────────┘
                    │ HTTPS
         ┌──────────┴──────────┐
         │   Alpaca Markets    │
         └─────────────────────┘

Shared: ~/.config/typhoon-terminal/
  ├── cache/typhoon_cache.db  (credentials + bar cache)
  └── .cred_salt              (encryption salt)

Shared: ~/.local/share/typhoon-terminal/
  └── account_registry.json   (MT5 imports)
```

## Features — Phase 1 (Implemented)

### Trading Core (20 commands)

| Feature | GUI | CLI | Command |
|---|---|---|---|
| Account info | ✅ | ✅ | Dashboard tab |
| Positions (interactive) | ✅ | ✅ | Positions tab |
| Orders (interactive) | ✅ | ✅ | Orders tab |
| Market order | ✅ | ✅ | `buy/sell SYM QTY` |
| Limit order | ✅ | ✅ | `limit buy/sell SYM QTY PRICE` |
| Stop order | ✅ | ✅ | `stop buy/sell SYM QTY PRICE` |
| Bracket order | ✅ | ✅ | `bracket buy/sell SYM QTY SL TP` |
| Close/Partial close | ✅ | ✅ | `close SYM [QTY]` |
| Close all | ✅ | ✅ | `closeall` |
| Cancel all | ✅ | ✅ | `cancelall` |
| Order history | ✅ | ✅ | `history [N]` |
| Watchlist + live quotes | ✅ | ✅ | Watchlist tab + `watch SYM` |
| Market clock | ✅ | ✅ | Dashboard tab |
| ASCII candlestick chart | N/A | ✅ | Chart tab + `chart SYM [TF]` |
| Custom timeframes | ✅ | ✅ | `tf TF` |
| MT5 CSV import | ✅ | ✅ | `import NAME /path.csv` |
| Multi-account aggregate | ✅ | ✅ | Accounts tab |
| Shared credentials | ✅ | ✅ | AES-256-GCM SQLite |

### TUI Layout (7 tabs)
1. **Dashboard** — Equity, buying power, positions summary, market status
2. **Chart** — ASCII candlestick chart with configurable symbol/timeframe
3. **Positions** — Live position table with P/L, qty, side, entry
4. **Orders** — Open orders with type, status, price
5. **Watchlist** — Live quotes for tracked symbols
6. **Accounts** — Imported MT5 CSV accounts with aggregate equity
7. **Command** — Log output

## Features — Phase 2 (Roadmap)

### Analytics & Risk (leverage SQLite cache shared with GUI)

| Feature | GUI | CLI | Priority | Notes |
|---|---|---|---|---|
| VaR calculation | ✅ | 🔲 | HIGH | Read from shared cache, compute in Rust |
| Risk dashboard | ✅ | 🔲 | HIGH | Margin %, exposure, drawdown |
| Position sizing | ✅ | 🔲 | HIGH | ATR-based, % risk, VaR-based |
| Screener results | ✅ | 🔲 | MED | Table output of scan criteria |
| News headlines | ✅ | 🔲 | MED | Finnhub/Alpaca news feed |
| Earnings calendar | ✅ | 🔲 | MED | Table of upcoming earnings |
| DARWIN analytics | ✅ | 🔲 | MED | Read from shared SQLite |
| Portfolio summary | ✅ | 🔲 | MED | Multi-account aggregate P/L |
| Export CSV | ✅ | 🔲 | LOW | Trade journal, positions |

### Chart Enhancements

| Feature | GUI | CLI | Priority | Notes |
|---|---|---|---|---|
| SMA/EMA overlay on ASCII chart | ✅ | 🔲 | MED | Compute in Rust, render as ASCII |
| Volume bars below chart | ✅ | 🔲 | LOW | Braille characters for density |
| MTF grid (2x2 ASCII charts) | ✅ | 🔲 | LOW | ratatui Layout::grid |

### Headless / Automation

| Feature | GUI | CLI | Priority | Notes |
|---|---|---|---|---|
| Headless backtest | ✅ | ✅ | DONE | `--backtest` flag |
| Webhook alerts | ✅ | 🔲 | HIGH | Pipe alerts to Discord/Slack |
| Scheduled commands | N/A | 🔲 | MED | Cron-like execution |
| JSON output mode | N/A | 🔲 | MED | `--json` flag for scripting |
| Position monitor daemon | N/A | 🔲 | HIGH | Background process, alert on margin/P&L |

## Binary

- **Size:** 6.5MB (release, stripped, LTO)
- **Dependencies:** reqwest, tokio, ratatui, crossterm, rusqlite, aes-gcm, serde
- **Platforms:** Linux, macOS, Windows (any terminal supporting ANSI escape codes)
- **No GUI deps:** No WebKitGTK, no Node.js, no Wasm — pure Rust

## Usage

```bash
# Interactive TUI
typhoon

# One-shot commands
typhoon --positions
typhoon --account
typhoon --accounts
typhoon --import-mt5 DARWIN_EUR:/path/to/statement.csv

# Headless backtest
typhoon --backtest --symbol AAPL --timeframe 1Day --fast 10 --slow 32

# With explicit keys (instead of shared credential storage)
typhoon --api-key PKXXX --secret-key SKXXX
```

## Consequences

- **Pro**: Full trading capability via SSH/VPS without display server
- **Pro**: Shared credentials + cache with GUI — no duplicate setup
- **Pro**: 6.5MB binary vs ~300MB GUI — perfect for deployment
- **Pro**: Scriptable with one-shot flags for automation
- **Con**: No GPU charting — ASCII only (acceptable for monitoring)
- **Con**: Phase 2 analytics require reading shared SQLite cache (not yet implemented)
