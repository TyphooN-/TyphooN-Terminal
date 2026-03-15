# TyphooN-Terminal Architecture Decision Record

## Why Rust + Tauri?

### The Requirements

TyphooN-Terminal replaces MetaTrader 5 as a local desktop trading terminal. It must:

1. **Render locally** — no browser-based solutions (TradingView web), no remote servers
2. **Chart with full indicator support** — candlesticks, multi-timeframe overlays, separate indicator panes
3. **Execute trades** — one-click order placement via draggable SL/TP lines
4. **Manage risk** — 4 order modes (Standard, Fixed, Dynamic, VaR), hedged martingale (TRIM/PROTECT)
5. **Be performant** — handle 10K+ bars, real-time updates, multiple indicator calculations without lag
6. **Be open-source** — long-term goal is a community-driven Godel Terminal-style product
7. **Run on Linux** — Arch/Hyprland with NVIDIA, Wayland-compatible

### Options Evaluated

#### 1. Python (rejected)

**Initial scaffold existed** (`Alpaca-Trading-System` repo) — risk engine in Python with alpaca-py SDK.

**Why rejected:**
- **No charting** — Python has matplotlib/plotly but they're not interactive trading charts. No draggable SL/TP lines, no real-time candlestick updates, no multi-pane layout
- **GUI frameworks are weak** — PyQt/PySide are heavy, slow to render financial charts, and have poor Wayland support. DearPyGui is better but still Python-speed for calculations
- **Two-process architecture** — would need Python backend + separate frontend for charting, communicating via IPC. Adds complexity, latency, and failure modes
- **GIL** — Python's Global Interpreter Lock means indicator calculations block the UI thread. Workarounds (multiprocessing, asyncio) add complexity without solving the core problem
- **Deployment** — distributing a Python app requires bundling the interpreter, managing pip dependencies, virtual environments. Users need Python installed or a 200MB+ frozen bundle
- **Memory** — Python's per-object overhead means 10K bars × 20 indicator series = significant memory pressure vs compiled languages

**What Python was good for:** rapid prototyping of the risk math. The Python scaffold served as a reference implementation for porting to Rust. The core math (TRIM, PROTECT, VaR, lot sizing) was validated in Python first, then ported.

#### 2. Electron (rejected)

**The "obvious" choice for desktop apps with web UIs.**

**Why rejected:**
- **Binary size** — Electron bundles Chromium. Minimum binary is ~150-200MB. TyphooN-Terminal's Tauri binary is ~10-15MB
- **Memory usage** — Electron apps consume 200-500MB RAM at baseline. Tauri uses the system's existing WebKitGTK (Linux) or WebView2 (Windows) — no extra browser process
- **Startup time** — Electron cold-starts in 2-5 seconds (loading Chromium). Tauri starts in <1 second
- **No Rust** — Electron's backend is Node.js (JavaScript). Risk calculations, margin math, and VaR need to be fast and correct. JavaScript's floating-point handling and lack of strong typing make financial math error-prone. Rust's type system and zero-cost abstractions are ideal for this domain
- **Security** — Electron's full Chromium has a massive attack surface. Tauri's webview is sandboxed with explicit IPC permissions, CSP headers, and strict input validation

#### 3. Qt/C++ (rejected)

**Traditional choice for professional trading terminals (like the original MetaTrader).**

**Why rejected:**
- **Development speed** — C++ is 3-5x slower to develop than Rust for the same functionality. Manual memory management, no package manager equivalent to Cargo, build system complexity (CMake)
- **Memory safety** — C++ has no borrow checker. Financial software with manual memory management is a liability. One use-after-free in a margin calculation could be catastrophic
- **Licensing** — Qt is dual-licensed (LGPL/commercial). The commercial license is expensive for an open-source project. LGPL has dynamic linking requirements that complicate distribution
- **Charting** — would need to build or buy a charting library. No equivalent to lightweight-charts that's free, battle-tested, and supports draggable price lines out of the box
- **Cross-platform** — Qt is cross-platform but requires significant per-platform work for native look. Tauri's web frontend renders identically everywhere

#### 4. Pure Rust GUI (egui/iced) (considered, deferred)

**Fully native Rust — no web technologies at all.**

**Why deferred (not rejected):**
- **egui** — immediate-mode GUI. Fast, simple, but financial charting is primitive. The `egui-candlestick-chart` crate exists but lacks draggable price lines, multi-pane layouts, and the polish of lightweight-charts. Building MT5-equivalent charting from scratch in egui would take months
- **iced** — retained-mode GUI. Flowsurface (a Rust trading terminal) uses iced successfully, but it's crypto-only and the charting code is tightly coupled to their specific use case. Porting would be as much work as building from scratch
- **Plotters** — the `plotters` crate can render candlesticks but is designed for static charts, not interactive trading. No real-time updates, no price line dragging, no crosshair interaction

**Future path:** Once TyphooN-Terminal's feature set is stable, migrating from Tauri (webview) to a pure Rust GUI (egui or iced) is viable. The Rust backend (risk engine, broker API, indicator math) is already written and would remain unchanged. Only the frontend rendering layer would change. This is the Godel Terminal long-term vision.

#### 5. Tauri + Rust (chosen)

**Rust backend + lightweight web frontend in a native window.**

**Why chosen:**

| Factor | Tauri + Rust |
|---|---|
| **Binary size** | ~10-15MB (uses system webview) |
| **Memory** | ~50-100MB (no bundled browser) |
| **Startup** | <1 second |
| **Backend language** | Rust — memory-safe, zero-cost abstractions, perfect for financial math |
| **Frontend** | HTML/CSS/JS — lightweight-charts for candlesticks (battle-tested, MIT license) |
| **Charting** | TradingView lightweight-charts: candlesticks, draggable price lines, multi-pane, real-time updates, 170KB |
| **IPC** | Tauri commands — type-safe, async, serialized via serde |
| **Cross-platform** | Linux (WebKitGTK), Windows (WebView2), macOS (WKWebView) |
| **License** | MIT/Apache-2.0 (Tauri), MIT (lightweight-charts), GPL-3.0 (TyphooN-Terminal) |
| **Proven pattern** | OpenAlgo Desktop, FastScalper-Tauri ship production trading apps with this stack |
| **Build system** | Cargo (Rust) + npm (frontend) — well-understood, reliable |
| **Testing** | Rust's built-in test framework for risk math (12 tests), frontend testable in browser |

### Architecture

```
┌─────────────────────────────────────────────────┐
│                  Tauri Window                     │
│  ┌─────────────────────────────────────────────┐ │
│  │          Frontend (HTML/CSS/JS)              │ │
│  │  ┌──────────┐ ┌──────────┐ ┌──────────────┐ │ │
│  │  │  Main    │ │  Fisher  │ │   Volume     │ │ │
│  │  │  Chart   │ │  Pane    │ │   Pane       │ │ │
│  │  │(candles, │ │(Ehlers   │ │(BetterVol)   │ │ │
│  │  │ overlays)│ │ Fisher)  │ │              │ │ │
│  │  └──────────┘ └──────────┘ └──────────────┘ │ │
│  │  lightweight-charts (MIT, 170KB)             │ │
│  └──────────────────────┬──────────────────────┘ │
│                         │ invoke()                │
│  ┌──────────────────────┴──────────────────────┐ │
│  │           Rust Backend (Tauri)               │ │
│  │  ┌──────────┐ ┌──────────┐ ┌──────────────┐ │ │
│  │  │  Risk    │ │ Margin   │ │   VaR        │ │ │
│  │  │  Engine  │ │  Math    │ │ Calculator   │ │ │
│  │  │(4 modes) │ │(TRIM/    │ │(inline StdDev│ │ │
│  │  │          │ │ PROTECT) │ │ inv. normal) │ │ │
│  │  └──────────┘ └──────────┘ └──────────────┘ │ │
│  │  ┌──────────┐ ┌──────────┐ ┌──────────────┐ │ │
│  │  │ Alpaca   │ │Martingale│ │  Discord     │ │ │
│  │  │ Broker   │ │  Engine  │ │  Webhooks    │ │ │
│  │  │(REST API)│ │(state    │ │              │ │ │
│  │  │          │ │ machine) │ │              │ │ │
│  │  └──────────┘ └──────────┘ └──────────────┘ │ │
│  └─────────────────────────────────────────────┘ │
└─────────────────────────────────────────────────┘
                         │
                    HTTPS/WSS
                         │
              ┌──────────┴──────────┐
              │   Alpaca Markets    │
              │  (Paper / Live)     │
              └─────────────────────┘
```

### Why Not Just Use MT5?

MetaTrader 5 works well for Darwinex CFD trading, but:

1. **Symbol coverage** — Darwinex offers ~100 instruments. Alpaca offers 11,000+ US stocks, ETFs, crypto, and options
2. **Broker lock-in** — MT5 is tied to the broker's server. Each broker instance needs its own MT5 installation. TyphooN-Terminal connects to any Alpaca account via API keys
3. **Linux support** — MT5 requires Wine on Linux. It works but has rendering glitches, no Wayland support, and Wine overhead. TyphooN-Terminal is native Linux
4. **Open source** — MT5 is proprietary (MetaQuotes). The MQL5 language is closed-source and can only run inside MT5. TyphooN-Terminal's risk engine is portable Rust that can be embedded anywhere
5. **Extensibility** — adding a new broker to TyphooN-Terminal means implementing one Rust trait. Adding a new indicator means writing one JavaScript function. In MT5, everything must be in MQL5 and compiled by MetaEditor

### The lightweight-charts Clarification

TyphooN-Terminal uses **TradingView's lightweight-charts library** — NOT the TradingView website/service.

- It's an MIT-licensed open-source JavaScript library (~170KB)
- Runs 100% locally in the Tauri webview — zero network calls to TradingView
- No TradingView account required
- The "TV" watermark was removed via `attributionLogo: false`
- Bar data comes from Alpaca's API, not TradingView's
- The library just renders candlesticks on an HTML canvas — it's a drawing library, not a service

It was chosen because:
- Battle-tested by millions of users (used in dozens of open-source trading apps)
- Supports draggable price lines (essential for SL/TP placement)
- Supports multiple chart instances for sub-panes (Fisher, Volume)
- Supports crosshair sync between panes
- Supports real-time bar updates
- 170KB — tiny footprint
- MIT license — no restrictions
