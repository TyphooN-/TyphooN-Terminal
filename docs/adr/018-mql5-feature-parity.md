# ADR-018: MQL5 Feature Parity Audit

**Status:** Complete
**Date:** 2026-03-15

## Context

TyphooN-Terminal is a port of TyphooN EA v1.420 from MQL5. A comprehensive audit of the original MQL5 codebase (2730 lines in TyphooN.mq5 + include files) was performed to verify all features have been ported to Rust/Tauri.

## MQL5 Source Files Audited

| File | Lines | Purpose |
|---|---|---|
| `Experts/TyphooN.mq5` | ~2730 | Main EA — buttons, dashboard, orders, martingale |
| `Include/Darwinex/DWEX Portfolio Risk Man.mqh` | ~300 | VaR calculation, StdDev, inverse normal |
| `Include/Orchard/RiskCalc.mqh` | ~50 | RiskLots, DoubleToTicks, NewBar |
| `Indicators/KAMA.mqh` | ~80 | Kaufman Adaptive MA |
| `Indicators/MultiKAMA.mqh` | ~150 | Multi-timeframe KAMA with global vars |
| `Indicators/ATR_Projection.mqh` | ~200 | ATR bands from multiple timeframes |
| `Indicators/PreviousCandleLevels.mqh` | ~180 | Previous bar high/low with Judas detection |
| `Indicators/EhlersFisherTransform.mqh` | ~100 | Ehlers Fisher oscillator |
| `Indicators/BetterVolume.mqh` | ~150 | Volume analysis (climax/churn/high/low) |

## Features Verified as Ported

| MQL5 Feature | Rust/Tauri Location | Status |
|---|---|---|
| 4 risk modes (Standard/Fixed/Dynamic/VaR) | `core/risk.rs` | Exact port |
| VaR with StdDev + inverse normal | `core/var.rs` | Exact port |
| TRIM zone (forward-looking margin math) | `core/margin.rs`, `strategies/martingale.rs` | Exact port |
| DEAD zone | `strategies/martingale.rs` | Exact port |
| PROTECT zone (urgency scaling) | `core/margin.rs`, `strategies/martingale.rs` | Exact port |
| Hard floor | `strategies/martingale.rs` | Exact port |
| Open MG (balanced hedge setup) | `main.rs` `open_martingale_hedge` | Exact port |
| Unwind mode (worst P/L first) | `strategies/martingale.rs` | Exact port |
| Break-even detection | `main.rs` `calculate_lots` | Exact port |
| `AdditionalRiskRatio` reduction | `core/risk.rs` | Exact port |
| 10 UI buttons | `index.html`, `main.js` | Exact layout |
| Dashboard (11 labels) | `main.js` `updateDashboard` | Exact port |
| Margin level with zone colors | `main.js` + `main.rs` `get_margin_info` | Exact port |
| Bar countdown timer | `main.js` `updateNextBarTime` | Single TF (MQL5 had multi-TF) |
| Discord webhooks | `notifications/mod.rs` | Exact port |
| Equity TP/SL protection | `main.rs` `set/check_equity_protection` | Newly ported |
| KAMA indicator | `main.js` `calcKAMA` | Exact algorithm |
| MultiKAMA (MTF) | `main.js` HTF KAMA projection | Exact port |
| ATR Projection (MTF) | `main.js` `calcATRProjection` | Exact port |
| Previous Candle Levels (MTF) | `main.js` `calcPrevCandleLevels` | Exact port with Judas |
| Ehlers Fisher Transform | `main.js` `calcEhlersFisher` | Exact port |
| BetterVolume | `main.js` `calcBetterVolume` | Exact port |
| Supply/Demand zones | `main.js` `calcSupplyDemandZones` | Exact port |
| SMA200 / SMA100 (MTF) | `main.js` MTF MA overlays | Exact port |

## Features Intentionally Not Ported

| MQL5 Feature | Reason |
|---|---|
| Filling mode selection (IOC/FOK/BOC) | Alpaca uses GTC exclusively |
| Async close with Sleep(100) polling | Alpaca REST is synchronous per request |
| Same-direction position blocking | Terminal allows multi-position (matches Dynamic/VaR) |
| NNFX indicator folder (30 indicators) | Reference-only in MQL5, not EA-driven |
| Window drag-to-move (chart objects) | Tauri window management handles this |
| MQL5 global variables for KAMA | Frontend uses module-scope state instead |
| PerformOrderCheck margin validation | Alpaca validates server-side |

## Consequences

- **Pro**: Full feature parity with MQL5 EA v1.420 for all trading-critical logic
- **Pro**: Several features EXCEED MQL5: backtester, optimizer, options chain, screener, command palette, WebSocket streaming
- **Con**: Some MQL5 UI polish (multi-TF countdowns, color-coded position labels) not yet matched
