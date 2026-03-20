# MarketWizardry Explorer → TyphooN-Terminal Migration

## Executive Summary

The four MarketWizardry explorers (VaR, ATR, EV, Crypto) are standalone web tools that detect statistical outliers in financial data. All four will be consolidated into TyphooN-Terminal as Ctrl+K command palette commands, powered by MT5/Darwinex bar data (via BarExporter EA) and Alpaca data. Once migration is complete, the explorers will be removed from MarketWizardry.org and replaced with a blog post directing users to TyphooN-Terminal.

---

## Data Pipeline

```
MT5 (Darwinex)                    Alpaca Markets
├── BarExporter.mq5 (10s timer)   ├── REST API (incremental fetch)
├── BarExporterFull.mq5 (seed)    └── WebSocket (live bars)
└── CSV → import_mt5_bars              │
         │                             │
         └──────────┬──────────────────┘
                    │
            SQLite Cache (TTBR binary + zstd)
            ├── mt5:EURUSD:1Hour (Darwinex data)
            ├── alpaca:AAPL:1Hour (Alpaca data)
            └── Symbol overlap: MT5 preferred, Alpaca fallback
                    │
            ┌───────┴────────┐
            │                │
    Outlier Analysis     Chart Rendering
    (Ctrl+K commands)    (MTF Grid + GPU)
            │
    ┌───────┼───────────┬────────────┐
    │       │           │            │
  VaR    ATR         EV          Crypto
Outliers Outliers  Outliers    Analysis
```

---

## Explorer Analysis: What Each Computes

### 1. VaR Explorer — Risk Outliers

**Method:** IQR (Interquartile Range) outlier detection on VaR/Ask ratio, grouped by industry sector.

**Steps:**
1. For each symbol: fetch 252 daily bars (1 year)
2. Compute daily returns: `r[i] = (close[i] - close[i-1]) / close[i-1]`
3. Compute VaR at 95% confidence: `VaR = mean(r) - 1.645 * stddev(r)`
4. Compute Risk Ratio: `risk_ratio = abs(VaR) / ask_price * 100`
5. Group by industry sector (GICS classification)
6. Per sector: compute Q1, Q3, IQR = Q3 - Q1
7. Outlier bounds: lower = Q1 - 1.5×IQR, upper = Q3 + 1.5×IQR
8. Flag symbols outside bounds as outliers

**Output:**
- Top 40 high-risk outliers (VaR/Ask > upper bound)
- Top 40 low-risk outliers (VaR/Ask < lower bound)
- Per-sector statistics (median, IQR, outlier count)
- Trading thesis for each outlier (why it matters)

**Example:** ORCL at 14.52% VaR/Ask in software sector (median 3.2%) = extreme risk outlier, likely post-earnings volatility.

### 2. ATR Explorer — Volatility Outliers

**Method:** IQR outlier detection on ATR/Ask ratio across D1 timeframe, grouped by industry.

**Steps:**
1. For each symbol: fetch 14 daily bars
2. Compute ATR(14) on D1 timeframe
3. Compute Volatility Ratio: `vol_ratio = ATR_D1 / ask_price * 100`
4. Group by industry sector
5. IQR outlier detection (same as VaR Explorer)
6. Flag extreme volatility symbols

**Output:**
- Ranked volatility table (highest ATR/Price first)
- Sector breakdown with IQR statistics
- Anomaly flags: news-driven spikes, technical breakdowns, earnings volatility

**Example:** DOGEUSD at 7.90% daily ATR = extreme volatility, expect 8-15% daily swings.

### 3. EV Explorer — Valuation Outliers

**Method:** MCap/EV ratio analysis with Z-score detection, combined with VaR for dual-metric screening.

**Steps:**
1. For each symbol: fetch fundamental data (market cap, enterprise value)
2. Compute MCap/EV ratio: `ratio = market_cap / enterprise_value * 100`
3. Interpret:
   - >150%: Fortress balance sheet (cash > debt), premium valuation
   - 80-120%: Normal (moderate leverage)
   - <50%: Heavy debt or deep value opportunity
   - Negative: Negative enterprise value (distress signal)
4. Z-score per sector for outlier detection
5. Cross-reference with VaR for dual-metric screening

**Output:**
- Ranked MCap/EV table with sector grouping
- Balance sheet strength scoring
- Dual-metric outliers: "High MCap/EV + Low VaR" (quality + stability)
- Scatter plot: MCap/EV vs VaR (visual quadrant analysis)

**Example:** AVGO at 967% MCap/EV = massive cash position, buyback potential. FCX at 77% = high debt, value trap or deep value.

### 4. Crypto Explorer — Crypto Risk Analysis

**Method:** Multi-timeframe volatility analysis with Z-score outliers and advanced ratio metrics.

**Steps:**
1. For each crypto pair: fetch bars across M1, D1, W1, MN1
2. Compute ATR at each timeframe
3. Compute VaR at 95% confidence
4. Classify risk tiers:
   - ATR/Price <2%: LOW risk
   - 2-5%: MEDIUM
   - 5-8%: HIGH
   - >8%: EXTREME
5. Advanced ratios:
   - VaR/ATR: Statistical vs market risk balance
   - Spread/VaR: Trading cost efficiency
   - ATR Monthly/Weekly ratio: Volatility acceleration (>4× = accelerating)
6. Z-score outlier detection across all crypto

**Output:**
- Volatility rankings table (all timeframes)
- Risk tier classification with color coding
- Advanced ratio analysis
- Correlation matrix across crypto pairs
- Event calendar (hard forks, mainnet launches)

---

## Gap Analysis: What Exists vs What's Missing

### Already Implemented in TyphooN-Terminal

| Feature | Command | Coverage |
|---|---|---|
| Per-position VaR | `VAR` (cmdVarBreakdown) | ✅ Individual position VaR |
| Portfolio risk heatmap | `RISKMAP` (cmdRiskMap) | ✅ Treemap by VaR weight |
| Position risk 360° | `RISK360` (cmdRisk360) | ✅ Sigma scenarios, Kelly |
| Correlation matrix | `CORR` (cmdCorrelation) | ✅ Position correlations |
| Monte Carlo | `MONTECARLO` (cmdMonteCarlo) | ✅ Risk of ruin simulation |
| ATR indicator | Chart overlay | ✅ ATR on all timeframes |
| Crypto market overview | `CRYPTO` (cmdCryptoMarket) | ✅ Top 50 coins, trending |
| Fear & Greed | `FEAR` (cmdFearGreed) | ✅ 30-day gauge |
| SEC filings | `SEC` (cmdSecFilings) | ✅ 10-K, 10-Q, 8-K |
| Insider trading | `INSIDER` (cmdInsider) | ✅ Form 4 filings |
| Analyst ratings | `ANR` (cmdAnalystRatings) | ✅ Recommendations |
| Congress trades | `CONGRESS` (cmdCongressTrades) | ✅ Recent transactions |
| Earnings calendar | `EARNINGS` (cmdEarnings) | ✅ Calendar view |
| Peer comparison | `PEERS` (cmdPeerComparison) | ✅ Fundamentals table |

### Missing — Needs Implementation

| Feature | Proposed Command | Description |
|---|---|---|
| **VaR Outlier Scanner** | `VAROUT` | IQR outlier detection on VaR/Ask across all symbols, sector-grouped |
| **ATR Outlier Scanner** | `ATROUT` | Volatility outlier detection on ATR/Price across all symbols |
| **EV Outlier Scanner** | `EVOUT` | MCap/EV ratio outliers with balance sheet scoring |
| **Crypto Risk Analysis** | `CRYPTORISK` | Multi-TF volatility tiers, advanced ratios, Z-score outliers |
| **Combined Outlier Report** | `OUTLIERS` | Unified report: VaR + ATR + EV outliers in one view |
| **Dual-Metric Screener** | `SCREEN` | Cross-reference VaR × EV × ATR for multi-factor outliers |

---

## Implementation Plan

### Phase 1: Core Outlier Engine (Rust backend)

New Tauri commands:
```
scan_var_outliers(symbols, period, confidence) → JSON
scan_atr_outliers(symbols, atr_period) → JSON
scan_ev_outliers(symbols) → JSON  (uses SEC/Finnhub fundamental data)
scan_crypto_risk(symbols) → JSON
```

Each returns:
```json
{
  "outliers": [
    {
      "symbol": "ORCL",
      "sector": "Technology",
      "metric": 14.52,
      "sector_median": 3.2,
      "sector_iqr": 2.1,
      "z_score": 5.4,
      "tier": "EXTREME",
      "direction": "high"
    }
  ],
  "sector_stats": { ... },
  "timestamp": "2026-03-20T17:00:00Z"
}
```

### Phase 2: Frontend Commands (Ctrl+K)

6 new command palette entries:
- `VAROUT` — VaR Outlier Scanner (floating window, sortable table)
- `ATROUT` — ATR Outlier Scanner
- `EVOUT` — EV Outlier Scanner
- `CRYPTORISK` — Crypto Risk Analysis
- `OUTLIERS` — Combined report (tabs for each scanner type)
- `SCREEN` — Multi-factor screener (filter by VaR + ATR + EV thresholds)

### Phase 3: MT5/Darwinex Integration

With BarExporter EA running:
1. Import all Darwinex bars into SQLite cache (`import_mt5_bars`)
2. Outlier scanners use MT5 data for Darwinex symbols, Alpaca for US stocks
3. Symbol overlap detection: compare MT5 vs Alpaca for same underlying
4. Full 9-timeframe analysis for every Darwinex symbol

### Phase 4: MarketWizardry.org Cleanup

1. Remove VaR Explorer, ATR Explorer, EV Explorer, Crypto Explorer pages
2. Remove `/var-explorer/`, `/atr-explorer/`, `/ev-explorer/`, `/crypto-explorer/` directories
3. Remove explorer JS files from `/js/`
4. Publish blog post: "Explorers are now in TyphooN-Terminal — here's how to use them"
5. Redirect explorer URLs to blog post

---

## Outlier Detection Method (IQR)

Used by all explorers. Standard statistical method for identifying outliers:

```
Given a set of values V for a metric (e.g., VaR/Ask ratio):

Q1 = 25th percentile of V
Q3 = 75th percentile of V
IQR = Q3 - Q1

Lower bound = Q1 - 1.5 × IQR
Upper bound = Q3 + 1.5 × IQR

Outliers = values < lower bound OR values > upper bound

For extreme outliers: use 3.0 × IQR instead of 1.5
```

This is applied per-sector so that tech stocks (naturally higher volatility) aren't compared against utilities (naturally lower volatility). Each sector has its own statistical distribution.

---

## Data Sources

| Source | Data | Used By | Status in Terminal |
|---|---|---|---|
| Alpaca REST API | US stock bars, quotes, orders | VaR, ATR scanners | ✅ Implemented |
| MT5/Darwinex (BarExporter) | Forex, CFD, crypto, index bars | All scanners | ✅ EA created, import ready |
| Finnhub | Company profiles, sector classification | Sector grouping | ✅ Implemented |
| SEC EDGAR | 10-K/10-Q filings (revenue, debt, cash) | EV scanner | ✅ Implemented |
| CoinGecko | Crypto market data, market cap | Crypto scanner | ✅ Implemented |
| FRED | Economic indicators | Macro overlay | ✅ Implemented |

---

## Timeline

| Phase | Work | Status |
|---|---|---|
| Phase 1 | Rust outlier engine: `calculate_atr`, `detect_outliers` (IQR), `scan_var_outliers`, `scan_atr_outliers`, `scan_crypto_risk` | ✅ **DONE** |
| Phase 2 | Frontend commands: VAROUT, ATROUT, EVOUT, CRYPTORISK, OUTLIERS, SCREEN | ✅ **DONE** |
| Phase 3 | MT5 data import: BarExporter.mq5, BarExporterFull.mq5, `import_mt5_bars`, `get_mt5_bars` | ✅ **DONE** |
| Phase 4 | MarketWizardry.org cleanup (see below) | Pending |

### Phase 4: MarketWizardry.org Cleanup Plan

1. **Create "Explorer Archive" menu item** in `/home/typhoon/git/MarketWizardry.org/`
   - New page: `/explorer-archive/index.html`
   - Contains all old explorer content (var-explorer, atr-explorer, ev-explorer, crypto-explorer)
   - Static archive — no longer updated

2. **Blog post at top of archive page + in blog section:**
   - Title: "MarketWizardry Explorers are now in TyphooN-Terminal"
   - Content: How to use TyphooN-Terminal's VAROUT, ATROUT, EVOUT, CRYPTORISK, OUTLIERS, SCREEN commands
   - Screenshots of each command output
   - Instructions for MT5 BarExporter setup (Darwinex data integration)
   - Link to TyphooN-Terminal GitHub

3. **Remove from main navigation:**
   - Remove VaR Explorer, ATR Explorer, EV Explorer, Crypto Explorer menu items
   - Replace with single "Explorer Archive" link
   - Remove `/js/var-explorer.js`, `/js/atr-explorer.js`, `/js/ev-explorer.js`, `/js/crypto-explorer.js`

4. **Redirect URLs:**
   - `/var-explorer/` → `/explorer-archive/#var`
   - `/atr-explorer/` → `/explorer-archive/#atr`
   - `/ev-explorer/` → `/explorer-archive/#ev`
   - `/crypto-explorer/` → `/explorer-archive/#crypto`
