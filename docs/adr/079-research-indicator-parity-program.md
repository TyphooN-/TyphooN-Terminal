# ADR-079: Research and Indicator Parity Program

**Status:** Implemented / continuing policy
**Date:** 2026-04-13
**Compacted:** 2026-05-27
**Updated:** 2026-06-18
**Supersedes:** obsolete parity implementation notes

## Context

The old ADR set had eighty-eight separate implementation-batch records for research surfaces and indicator primitives. That made the ADR directory noisy and hid the actual architecture behind execution diary entries. Those records were useful while the work was being executed, but they are not useful as top-level onboarding material.

This ADR is now the single human-readable record for the research/indicator expansion program. Git history preserves deleted implementation-batch detail if archaeology is ever needed.

## Decision

1. Keep one architecture ADR instead of one ADR per scraping/indicator implementation batch.
2. Treat product research/data surfaces and indicator primitives as one program with explicit classification:
   - External-terminal-style research/data surfaces are product parity targets.
   - Library-compatible indicator primitives are indicator/research expansion, not evidence that an upstream product had that feature.
   - Research-packet and egui-window coverage are considered implemented when cached data, fetch commands, and a visible window/pane exist.
   - Chart overlays remain governed by the current chart/indicator architecture; do not create a new ADR per overlay unless it changes the architecture.
3. New research/indicator work should update this ADR or a current feature ADR. Do not create another implementation-batch ADR.

## Current architecture

- Research and indicator data models live primarily under `typhoon-engine/src/core/research.rs` and related broker/cache plumbing.
- UI surfaces live in the native egui app with command-palette entry points and cache-first fetch behavior.
- LAN/web/client reuse should expose cached research packets rather than duplicating provider fetches per client.
- Provider-entitled or paid-feed-only surfaces are tracked as data-gated roadmap work, not active implementation gaps.

## Compacted implementation inventory

| Old ADR | Compacted topic | Old status | Old date |
| --- | --- | --- | --- |
| 108 | Research windows and bulk scrape | Implemented | 2026-04-13 |
| 109 | Research/data surface group — Dividends, Forward Earnings, Ratings, Treasury Curve | Implemented | 2026-04-13 |
| 110 | Research/data surface group — Financials, Management, CFTC Positioning | Implemented | 2026-04-14 |
| 111 | Research/data surface group — Splits, ETF Holdings, Analyst Ratings, ESG, Index Membership + AI Chat Overhaul | Implemented | 2026-04-14 |
| 112 | Research/data surface group — INS/HDS/FLOAT/HP/EPS | Recorded |  |
| 113 | Research/data surface group — WEI/MOV/INDU/CACS/WACC | Recorded |  |
| 114 | Research/data surface group — WCR / BETA / DDM / RV / FIGI | Accepted | 2026-04-14 |
| 115 | Research/data surface group — HRA / DCF / SVM / OMON / IVOL | Accepted | 2026-04-14 |
| 116 | Research/indicator group — SEAG / COR / TRA / TECH / SKEW | Accepted | 2026-04-14 |
| 117 | Research/data surface group — LEV / ACRL / RVOL / FCFY / SHRT | Accepted | 2026-04-14 |
| 118 | Research/indicator group — ALTZ / PTFS / VOLE / EPSB / PTD | Accepted | 2026-04-14 |
| 119 | Research/data surface group — MNGR / DIVG / EARM / SECTR / UPDM | Accepted | 2026-04-14 |
| 120 | Research/data surface group — MOM / LIQ / BREAK / CCRL / CREDIT | Accepted | 2026-04-14 |
| 121 | Research/data surface group — GROWM / FLOW / REGIME / RELVOL / MARGINS | Accepted | 2026-04-15 |
| 122 | Research/data surface group — VAL / QUAL / RISK / INSSTRK / COVG | Accepted | 2026-04-15 |
| 123 | Research/data surface group — VRK / QRK / RRK / RELEPSGR / PEAD | Accepted | 2026-04-15 |
| 124 | Research/data surface group — SIZEF / MOMF / PEADRANK / FQM / REVRANK | Accepted | 2026-04-15 |
| 125 | Research/data surface group — LEVRANK / OPERANK / FQMRANK / LIQRANK / SURPSTK | Accepted | 2026-04-15 |
| 126 | Research/data surface group — DVDRANK / EARMRANK / UPDGRANK / GY / DES | Accepted | 2026-04-15 |
| 127 | Research/indicator group — DVDYIELDRANK / SHRANK / ATRANN / DDHIST / PRICEPERF | Accepted | 2026-04-15 |
| 128 | Research/data surface group — BETARANK / PEGRANK / FHIGHLOW / RVCONE / CALPB | Accepted | 2026-04-15 |
| 129 | Quant-statistics group — RETSKEW / RETKURT / TAILR / RUNLEN / DAYRANGE | Accepted | 2026-04-15 |
| 131 | Quant-statistics group — AUTOCOR / HURST / HITRATE / GLASYM / VOLRATIO | Accepted | 2026-04-15 |
| 132 | Quant-statistics group — DRAWUP / GAPSTATS / VOLCLUSTER / CLOSEPLC / MRHL | Accepted | 2026-04-15 |
| 133 | Quant-statistics group — DOWNVOL / SHARPR / EFFRATIO / WICKBIAS / VOLOFVOL | Accepted | 2026-04-15 |
| 134 | Quant-statistics group — CALMAR / ULCER / VARRATIO / AMIHUD / JBNORM | Accepted | 2026-04-15 |
| 135 | Quant-statistics group — OMEGA / DFA / BURKE / MONTHSEAS / ROLLSPRD | Accepted | 2026-04-15 |
| 136 | Quant-statistics group — PARKINSON / GKVOL / RSVOL / CVAR / DOWEFFECT | Accepted | 2026-04-15 |
| 137 | Quant-statistics group — STERLING / KELLYF / LJUNGB / RUNSTEST / ZERORET | Accepted | 2026-04-15 |
| 138 | Quant-statistics group — PSR / ADF / MNKENDALL / BIPOWER / DDDUR | Accepted | 2026-04-15 |
| 139 | Quant-statistics group — HILLTAIL / ARCHLM / PAINRATIO / CUSUM / CFVAR | Accepted | 2026-04-15 |
| 140 | Quant-statistics group — ENTROPY / RACHEV / GPR / PACF / APEN | Accepted | 2026-04-16 |
| 141 | Quant-statistics group — UPR / LEVEREFF / DRAWDAR / VARHALF / GINI | Accepted | 2026-04-16 |
| 142 | Quant-statistics group — SAMPEN / PERMEN / RECFACT / KPSS / SPECENT | Accepted | 2026-04-16 |
| 143 | Quant-statistics group — ROBVOL / RENYIENT / RETQUANT / MSENT / EWMAVOL | Accepted | 2026-04-16 |
| 144 | Quant-statistics group — KSNORM / ADTEST / LMOM / KYLELAM / PEAKOVER | Accepted | 2026-04-16 |
| 145 | Quant-statistics group — HIGUCHI / PICKANDS / KAPPA3 / LYAPUNOV / RANKAC | Accepted | 2026-04-16 |
| 146 | Quant-statistics group — BNSJUMP / PPROOT / MFDFA / HILLKS / TSI | Accepted | 2026-04-16 |
| 147 | Quant-statistics group — GARCH11 / SADF / CORDIM / SKSPEC / AUTOMI | Accepted | 2026-04-16 |
| 149 | Quant-statistics group — DURBINWATSON / BDSTEST / BREUSCHPAGAN / TURNPTS / PERIODOGRAM | Accepted | 2026-04-16 |
| 150 | Quant-statistics group — MCLEODLI / OUFIT / GPH / BURGSPEC / KENDALLTAU | Accepted | 2026-04-17 |
| 151 | Research/indicator group — SQUEEZE / SQUEEZERANK / BBSQUEEZE / DONCHIAN / KAMA | Accepted | 2026-04-17 |
| 152 | Research/indicator group — ICHIMOKU / SUPERTREND / KELTNER / FISHER / AROON | Accepted | 2026-04-17 |
| 153 | Research/indicator group — ADX / CCI / CMF / MFI / PSAR | Accepted | 2026-04-17 |
| 154 | Research/indicator group — VORTEX / CHOP / OBV / TRIX / HMA | Accepted | 2026-04-17 |
| 155 | Research/indicator group — PPO / DPO / KST / ULTOSC / WILLR | Accepted | 2026-04-17 |
| 156 | Research/indicator group — MASS / CHAIKOSC / KLINGER / STOCHRSI / AWESOME | Accepted | 2026-04-17 |
| 158 | Research/data surface group — EFI / EMV / NVI / PVI / COPPOCK | Accepted | 2026-04-17 |
| 159 | Research/indicator group — CMO / QSTICK / DISPARITY / BOP / SCHAFF | Accepted | 2026-04-17 |
| 160 | Research/indicator group — STOCH / MACD / VWAP / MCGD / RWI | Accepted | 2026-04-17 |
| 161 | Research/indicator group — DEMA / TEMA / LINREG / PIVOTS / HEIKIN | Accepted | 2026-04-17 |
| 163 | Research/indicator group — ALMA / ZLEMA / ELDERRAY / TSF / RVI | Accepted | 2026-04-17 |
| 164 | Research/indicator group — TRIMA / T3 / VIDYA / SMI / PVT | Accepted | 2026-04-17 |
| 165 | Research/data surface group — AC / CHVOL / BBWIDTH / ELDERIMP / RMI | Accepted | 2026-04-17 |
| 167 | Research/data surface group — SMMA / ALLIGATOR / CRSI / SEB / IMI | Accepted | 2026-04-17 |
| 168 | Research/indicator group — GMMA / MAENV / ADL / VHF / VROC | Accepted | 2026-04-17 |
| 169 | Research/data surface group — KDJ / QQE / PMO / CFO / TMF | Accepted | 2026-04-17 |
| 170 | Research/indicator group — FRACTALS / IFT_RSI / MAMA / COG / DIDI | Accepted | 2026-04-17 |
| 171 | Research/indicator group — DEMARKER / GATOR / BW_MFI / VWMA / STDDEV | Accepted | 2026-04-17 |
| 172 | Research/indicator group — WMA / RAINBOW / MESA_SINE / FRAMA / IBS | Accepted | 2026-04-17 |
| 173 | Research/indicator group — LAGUERRE_RSI / ZIGZAG / PGO / HT_TRENDLINE / MIDPOINT | Accepted | 2026-04-17 |
| 174 | Research/indicator group — MASSINDEX / NATR / TTM_SQUEEZE / FORCE_INDEX / TRANGE | Accepted | 2026-04-17 |
| 175 | Research/indicator group — LINEARREG_SLOPE / HT_DCPERIOD / HT_TRENDMODE / ACCBANDS / STOCHF | Accepted | 2026-04-17 |
| 176 | Indicator primitive group — LINEARREG / LINEARREG_ANGLE / HT_DCPHASE / HT_SINE / HT_PHASOR | Accepted | 2026-04-17 |
| 177 | Indicator primitive group — MIDPRICE / APO / MOM / SAREXT / ADXR | Accepted | 2026-04-18 |
| 178 | Indicator primitive group — AVGPRICE / MEDPRICE / TYPPRICE / WCLPRICE / VARIANCE | Accepted | 2026-04-18 |
| 179 | Indicator primitive group — PLUS_DI / MINUS_DI / PLUS_DM / MINUS_DM / DX | Accepted | 2026-04-18 |
| 180 | Indicator primitive group — ROC / ROCP / ROCR / ROCR100 / CORREL | Accepted | 2026-04-18 |
| 181 | Indicator primitive group — MIN / MAX / MINMAX / MININDEX / MAXINDEX | Accepted | 2026-04-18 |
| 182 | Indicator primitive group — BBANDS / AD / ADOSC / SUM / LINEARREG_INTERCEPT | Accepted | 2026-04-18 |
| 183 | Indicator primitive group — AROONOSC / MINMAXINDEX / MACDEXT / MACDFIX / MAVP | Accepted | 2026-04-18 |
| 184 | Indicator primitive group — CDLDOJI / CDLHAMMER / CDLSHOOTINGSTAR / CDLENGULFING / CDLHARAMI | Accepted | 2026-04-18 |
| 185 | Indicator primitive group — CDLMORNINGSTAR / CDLEVENINGSTAR / CDL3BLACKCROWS / CDL3WHITESOLDIERS / CDLDARKCLOUDCOVER | Accepted | 2026-04-18 |
| 186 | Indicator primitive group — CDLPIERCING / CDLDRAGONFLYDOJI / CDLGRAVESTONEDOJI / CDLHANGINGMAN / CDLINVERTEDHAMMER | Accepted | 2026-04-19 |
| 187 | Indicator primitive group — CDLHARAMICROSS / CDLLONGLEGGEDDOJI / CDLMARUBOZU / CDLSPINNINGTOP / CDLTRISTAR (research-layer, chart overlay deferred) | Implemented (research layer); chart overlays governed by ADR-079 | 2026-04-19 |
| 188 | Chart-Drawing Parity Deferred — Research-Packet-First | Accepted | 2026-04-19 |
| 189 | Quant-statistics and indicator research surfaces | Accepted | 2026-04-20 |
| 190 | Additional candlestick research surfaces | Accepted | 2026-04-20 |
| 191 | Complex candlestick research surfaces | Accepted | 2026-04-20 |
| 192 | Additional multi-bar candlestick research surfaces | Accepted | 2026-04-20 |
| 193 | Stateful candlestick research surfaces | Accepted | 2026-04-21 |
| 194 | Final candlestick research surfaces | Accepted | 2026-04-21 |
| 195 | Deferred benchmark and peer-relative momentum surfaces | Accepted | 2026-04-21 |
| 196 | Deferred sector liquidity and benchmark-link ranks | Accepted | 2026-04-21 |
| 197 | Operating-rank delta, dividend accrual, EPS accrual, and volatility-risk-premium surfaces | Accepted | 2026-04-21 |
| 198 | Short-rank delta and short-interest history | Accepted | 2026-04-21 |
| 199 | Insider-concentration research surface | Accepted | 2026-04-22 |
| 200 | GPU/CPU chart indicators for CMO, QSTICK, DISPARITY, BOP, STDDEV | Accepted | 2026-04-22 |

## Maintenance rule

If a future parity feature changes data ownership, cache layout, broker API contracts, or chart-overlay architecture, write a focused ADR for that architectural decision. If it merely adds another research packet, scanner column, indicator primitive, or egui view under the same architecture, update this ADR and the user-facing docs instead.
