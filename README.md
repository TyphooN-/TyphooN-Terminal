# TyphooN-Terminal

A native desktop trading terminal with full risk management, multi-timeframe charting, and hedged martingale support — built in Rust/Tauri for Alpaca Markets.

**Website:** [MarketWizardry.org](https://www.marketwizardry.org/)

---

## Features

| Feature | Description |
|---|---|
| **Charting** | Candlestick charts with 10K+ bar support, multi-timeframe indicator overlays, separate indicator panes |
| **Risk Management** | 4 order modes: Standard (% risk), Fixed lots, Dynamic (min-balance scaling), VaR (percent/notional) |
| **Hedged Martingale** | Forward-looking TRIM, dynamic PROTECT, Open MG one-click setup, equity TP, unwind — full port of TyphooN EA v1.420 |
| **Order Placement** | Draggable SL/TP lines on chart, one-click order with automatic lot calculation, keyboard shortcuts |
| **Multi-Account** | Save/load multiple Alpaca accounts (paper + live), secure local credential storage |
| **Indicators** | Full NNFX system ported from MQL5 + standard indicators (RSI, MACD, Bollinger, etc.) |

---

## NNFX Indicator System

Ported from the [MQL5-NNFX-Risk_Management_System](https://github.com/TyphooN-/MQL5-NNFX-Risk_Management_System) with exact visual parity.

### Main Chart (enabled by default)

| Indicator | Source | Description |
|---|---|---|
| **MultiKAMA** (10/2/30) | KAMA.mqh / MultiKAMA.mqh | Kaufman Adaptive MA from multiple timeframes, white, width 2 |
| **200 SMA** | Standard | Yellow, width 1 |
| **Previous Candle Levels** | PreviousCandleLevels.mqh | MTF previous bar high/low — white (H1/H4), magenta (D1/W1) |
| **ATR Projection** (14) | ATR_Projection.mqh | MTF open ± ATR bands — solid yellow, width 2 |
| **Supply/Demand Zones** | Custom | Filled rectangles at impulse move origins — green (demand), red (supply) |

### Separate Panes (enabled by default)

| Indicator | Source | Description |
|---|---|---|
| **Ehlers Fisher Transform** (32) | EhlersFisherTransform.mqh | Color-changing line (green bullish / red bearish) + gray signal line |
| **BetterVolume** | Custom | Volume histogram colored by price action (climax/churn/high/low) |

### MTF MA Grid

| Row | Description |
|---|---|
| SMA200 | Price above/below 200 SMA on H1/H4/D1/W1 — green/red dots |
| KAMA | Price above/below KAMA on each timeframe |
| Fisher | Fisher Transform bullish/bearish state per timeframe |

### Standard Indicators (disabled by default)

EMA (50/200), SMA (50), DEMA (21), RSI (14), MACD (12/26/9), Bollinger (20), ATR (14), VWAP, RVOL (10), Volume

---

## Risk Engine

Full port of TyphooN EA v1.420 risk management from MQL5 to Rust:

| Module | Description |
|---|---|
| **margin.rs** | Forward-looking TRIM, PROTECT urgency, spread tolerance, usable margin with buffer |
| **risk.rs** | All 4 order modes (Standard/Fixed/Dynamic/VaR), RiskLots calculation, lot normalization |
| **var.rs** | VaR calculation with inline StdDev, inverse cumulative normal, configurable confidence |
| **position.rs** | Hedge/bias tracking, break-even detection, SL/TP P/L, risk/reward ratio |
| **martingale.rs** | State machine (OFF/LONG/SHORT/UNWIND), TRIM/PROTECT decisions, Open MG sizing, equity TP |

12 unit tests covering margin math, lot sizing, and VaR calculations.

---

## Keyboard Shortcuts

| Key | Action |
|---|---|
| `b` | Buy Lines (SL = low, TP = high) |
| `s` | Sell Lines (SL = high, TP = low) |
| `d` | Destroy Lines |
| `t` | Open Trade |
| `m` | Martingale mode toggle |
| `o` | Open MG |
| `c` | Close All |
| `p` | Close Partial |
| `Esc` | Clear SL/TP lines |

---

## Architecture

**Rust backend (Tauri 2.0)** — risk engine, Alpaca REST API, margin math, VaR, martingale state machine.

**JavaScript frontend** — TradingView lightweight-charts (MIT, 170KB), HTML/CSS UI with 10-button panel, 11-label dashboard, indicator config panel.

See [ARCHITECTURE.md](ARCHITECTURE.md) for the full decision record (why Rust/Tauri vs Python, Electron, Qt/C++, pure Rust GUI).

See [INDICATOR_PORTING.md](INDICATOR_PORTING.md) for lessons learned porting MQL5 indicators to JavaScript.

---

## Building

### Prerequisites

- Rust (latest stable)
- Node.js 18+
- Tauri CLI: `cargo install tauri-cli`
- Linux: `libwebkit2gtk-4.1-dev`, `libgtk-3-dev`, `libayatana-appindicator3-dev`

### Development

```bash
cd frontend && npm install
cd ../src-tauri && cargo tauri dev
```

On Hyprland/NVIDIA:
```bash
WEBKIT_DISABLE_DMABUF_RENDERER=1 WEBKIT_DISABLE_COMPOSITING_MODE=1 GDK_BACKEND=x11 cargo tauri dev
```

### Production Build

```bash
cargo tauri build
```

---

## Broker

Currently supports [Alpaca Markets](https://alpaca.markets/) (stocks, ETFs, crypto, options):

- Paper and live trading accounts
- REST API for orders, positions, account info
- Historical bar data with IEX/SIP feed support
- Multi-timeframe data fetching for MTF indicators

---

## License

GNU General Public License v3.0

---

## Disclaimer

This software is provided for educational and research purposes. Trading involves risk. Past performance does not guarantee future results. Always test with paper trading before using real funds.
