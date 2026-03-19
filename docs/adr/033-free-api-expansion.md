# ADR-033: Free API Expansion — Data Sources Research

**Status:** Research Complete
**Date:** 2026-03-19

## Context

Comprehensive research into free APIs and government data sources that could further expand TyphooN-Terminal's capabilities beyond the 21 data sources already integrated.

## Currently Integrated (21 Sources)

| Source | Auth | Features |
|---|---|---|
| Alpaca Markets | API key | Trading, bars, quotes, news, options, corporate actions, portfolio history, clock |
| SEC EDGAR | User-Agent | Filings, fundamentals, 13F, insider trades, Form 4 |
| FRED | API key | Economic data (Fed Funds, CPI, GDP, Treasury yields, VIX, M2) |
| Finnhub | API key | Analyst ratings, price targets, short interest, insider sentiment, IPO, earnings, economic calendar |
| FMP | API key | Analyst estimates, financial ratios, DCF |
| Alpha Vantage | API key | Earnings surprises, company overview |
| CoinGecko | None | Crypto market data, trending, sparklines |
| ECB | None | Forex daily rates (XML) |
| House Stock Watcher | None | Congressional stock trades |
| Yahoo Finance | None | World equity indices (14 major) |
| Treasury.gov | None | Daily treasury yield rates |
| alternative.me | None | Crypto Fear & Greed Index |
| whale-alert.io | Free key | Large crypto transactions |
| Reddit JSON | None | WSB/investing post search |
| FINRA RegSHO | None | Daily short sale volume |
| Pushover | User key | Mobile push notifications |
| ntfy.sh | None | Free push notifications |
| Anthropic | API key | AI chat (Claude) |
| OpenAI | API key | AI chat (GPT) |
| QuiverQuant | API key | Congress trading (paid tier) |
| Matrix | None | Community chat protocol |

## Researched — Available for Future Integration

### Tier 1: High Value, Government Open Data (Free, Taxpayer-Funded)

| Source | Auth | Data | Trading Use |
|---|---|---|---|
| **EIA** (Energy Information Administration) | Free API key | Crude oil/NG inventories, petroleum supply | Weekly inventory reports move energy markets |
| **CFTC COT** (Commitments of Traders) | None (Socrata API) | Commercial/speculative positioning in futures | Best macro signal for commodities/forex |
| **USDA NASS** | Free API key | Crop production, acreage, livestock, WASDE | Agricultural commodity forecasting |
| **BLS v2** | Free registration | Detailed employment, PPI by commodity, CPI components | Granular labor data before FRED aggregates |
| **Census Bureau** | Free key | Trade by country/commodity, retail sales, housing | Track US-China trade flows |
| **Treasury TIC** | None (text file) | Foreign holdings of US Treasuries by country | Macro: Japan/China reducing = Treasury weakness |
| **NOAA/NWS** | None | Severe weather alerts, drought data, hurricane tracking | Agricultural commodities + energy trading |
| **USPTO** | None | Patent filings, application velocity | Innovation indicator by company |

### Tier 2: Exchange & Market Structure Data

| Source | Auth | Data | Trading Use |
|---|---|---|---|
| **SqueezMetrics** | Free account | DIX (dark pool sentiment), GEX (gamma exposure) | Best short-term S&P 500 indicator |
| **OCC** | None (scrape) | Daily options volume, P/C ratios by account type | Retail vs institutional positioning |
| **FINRA TRACE** | Free registration | Corporate bond trade data | Credit market stress detection |
| **CME FedWatch** | Free (scrape or pyfedwatch) | Fed rate probability | Rate changes move everything |

### Tier 3: Crypto On-Chain (Free)

| Source | Auth | Data | Trading Use |
|---|---|---|---|
| **DeFi Llama** | None | TVL (7,210 protocols), DEX volume, fees, stablecoins | DeFi flow tracking, stablecoin expansion signal |
| **Mempool.space** | None | BTC fees, LN stats, mining hashrate, pool distribution | Network congestion, miner capitulation detection |
| **Etherscan** | Free key | Wallet balances, token transfers, gas prices | Whale wallet tracking |
| **Solscan** | Free key | SOL wallet tracking, DeFi activity | SOL whale monitoring |

### Tier 4: Alternative Data & Prediction Markets

| Source | Auth | Data | Trading Use |
|---|---|---|---|
| **ApeWisdom** | None | Reddit top mentioned tickers with counts | WSB momentum (better than raw Reddit) |
| **Polymarket** | None | 1,700+ prediction markets (elections, Fed, events) | Real-money sentiment on macro events |
| **Kalshi** | None | CFTC-regulated event contracts | Economic event probability |
| **Google Trends** (pytrends) | None | Search volume for any term | Retail attention proxy |

### Tier 5: Macro/International

| Source | Auth | Data | Trading Use |
|---|---|---|---|
| **World Bank** | None | 1,400+ indicators, 200+ countries | EM GDP, global trade |
| **IMF** | None | World Economic Outlook, fiscal monitors | Global macro regime |

## Killer Feature Ideas (Would Go Viral)

1. **Net Liquidity Dashboard** — `WALCL - TGA - RRP` from FRED, overlaid on S&P/BTC. The most requested TradingView indicator.
2. **CFTC COT Positioning Overlay** — Commercial vs speculative on commodity/forex charts. No free terminal does this.
3. **SqueezMetrics DIX/GEX** — Dark pool sentiment + gamma exposure. Hedge funds pay for this.
4. **DeFi Llama TVL Dashboard** — Stablecoin supply + protocol flows. Leading crypto indicator.
5. **Prediction Market Odds** — Polymarket/Kalshi alongside charts. Real money > surveys.
6. **Weather × Commodities** — NOAA severe weather next to agricultural prices. Zero terminals do this.
7. **EIA Inventory Alerts** — Countdown to weekly petroleum/NG reports with historical surprise data.

## Decision

Features in "Currently Integrated" are shipped. Tier 1-5 features are documented for future implementation when demand warrants. The highest-impact additions would be Net Liquidity, CFTC COT, and DeFi Llama — all require zero API keys.
