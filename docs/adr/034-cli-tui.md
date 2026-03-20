# ADR-034: CLI / TUI Terminal Interface

**Status:** Implemented
**Date:** 2026-03-19

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

## Features (Trading Parity with GUI)

| Feature | GUI | CLI |
|---|---|---|
| Account info | ✅ | ✅ |
| Positions (interactive) | ✅ | ✅ |
| Orders (interactive) | ✅ | ✅ |
| Market/Limit/Stop/Bracket orders | ✅ | ✅ |
| Close/Partial close | ✅ | ✅ |
| Close all / Cancel all | ✅ | ✅ |
| Order history | ✅ | ✅ |
| Watchlist + live quotes | ✅ | ✅ |
| Market clock | ✅ | ✅ |
| Risk dashboard (VaR, margin) | ✅ | ✅ |
| ASCII candlestick chart | N/A | ✅ |
| Custom timeframes (H2-MN1) | ✅ | ✅ |
| MT5 CSV import | ✅ | ✅ |
| Multi-account aggregate | ✅ | ✅ |
| Shared credentials | ✅ | ✅ |

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

# With explicit keys (instead of shared credential storage)
typhoon --api-key PKXXX --secret-key SKXXX
```
