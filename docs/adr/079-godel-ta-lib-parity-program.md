# ADR-079: Godel / TA-Lib Parity Program

**Status:** Implemented / continuing policy
**Date:** 2026-04-13
**Compacted:** 2026-05-27
**Supersedes:** obsolete parity-round notes

## Context

The old ADR set had eighty-eight separate Godel/TA-Lib parity round records. That made the ADR directory noisy and hid the actual architecture behind implementation diary entries. Those records were useful while the work was being executed, but they are not useful as top-level onboarding material.

This ADR is now the single human-readable record for the parity program. Git history preserves the deleted per-round detail if archaeology is ever needed.

## Decision

1. Keep one parity ADR instead of one ADR per scraping/indicator round.
2. Treat Godel parity and TA-Lib parity as one program with explicit classification:
   - Godel-documented research/data surfaces are product parity targets.
   - TA-Lib-only primitives are indicator/research expansion, not evidence that Godel had that feature.
   - Research-packet and egui-window coverage are considered implemented when cached data, fetch commands, and a visible window/pane exist.
   - Chart overlays remain governed by the current chart/indicator architecture; do not create a new ADR per overlay unless it changes the architecture.
3. New parity work should update this ADR or a current feature ADR. Do not create another "round N" ADR.

## Current architecture

- Research and parity data models live primarily under `engine/src/core/research.rs` and related broker/cache plumbing.
- UI surfaces live in the native egui app with command-palette entry points and cache-first fetch behavior.
- LAN/web/client reuse should expose cached parity packets rather than duplicating provider fetches per client.
- Provider-entitled or paid-feed-only surfaces are tracked as data-gated roadmap work, not active implementation gaps.

## Compacted round inventory

| Old ADR | Compacted topic | Old status | Old date |
| --- | --- | --- | --- |
| 108 | Godel Parity: Research Windows & Bulk Scrape | Implemented | 2026-04-13 |
| 109 | Godel Parity Round 2: Dividends, Forward Earnings, Ratings, Treasury Curve | Implemented | 2026-04-13 |
| 110 | Godel Parity Round 3: Financials, Management, CFTC Positioning | Implemented | 2026-04-14 |
| 111 | Godel Parity Round 4: Splits, ETF Holdings, Analyst Ratings, ESG, Index Membership + AI Chat Overhaul | Implemented | 2026-04-14 |
| 112 | Godel Parity Round 5 — INS/HDS/FLOAT/HP/EPS | Recorded |  |
| 113 | Godel Parity Round 6 — WEI/MOV/INDU/CACS/WACC | Recorded |  |
| 114 | Godel Parity Round 7 — WCR / BETA / DDM / RV / FIGI | Accepted | 2026-04-14 |
| 115 | Godel Parity Round 8 — HRA / DCF / SVM / OMON / IVOL | Accepted | 2026-04-14 |
| 116 | TA-Lib + Godel Parity Round 9 — SEAG / COR / TRA / TECH / SKEW | Accepted | 2026-04-14 |
| 117 | Godel Parity Round 10 — LEV / ACRL / RVOL / FCFY / SHRT | Accepted | 2026-04-14 |
| 118 | TA-Lib + Godel Parity Round 11 — ALTZ / PTFS / VOLE / EPSB / PTD | Accepted | 2026-04-14 |
| 119 | Godel Parity Round 12 — MNGR / DIVG / EARM / SECTR / UPDM | Accepted | 2026-04-14 |
| 120 | Godel Parity Round 13 — MOM / LIQ / BREAK / CCRL / CREDIT | Accepted | 2026-04-14 |
| 121 | Godel Parity Round 14 — GROWM / FLOW / REGIME / RELVOL / MARGINS | Accepted | 2026-04-15 |
| 122 | Godel Parity Round 15 — VAL / QUAL / RISK / INSSTRK / COVG | Accepted | 2026-04-15 |
| 123 | Godel Parity Round 16 — VRK / QRK / RRK / RELEPSGR / PEAD | Accepted | 2026-04-15 |
| 124 | Godel Parity Round 17 — SIZEF / MOMF / PEADRANK / FQM / REVRANK | Accepted | 2026-04-15 |
| 125 | Godel Parity Round 18 — LEVRANK / OPERANK / FQMRANK / LIQRANK / SURPSTK | Accepted | 2026-04-15 |
| 126 | Godel Parity Round 19 — DVDRANK / EARMRANK / UPDGRANK / GY / DES | Accepted | 2026-04-15 |
| 127 | TA-Lib + Godel Parity Round 20 — DVDYIELDRANK / SHRANK / ATRANN / DDHIST / PRICEPERF | Accepted | 2026-04-15 |
| 128 | Godel Parity Round 21 — BETARANK / PEGRANK / FHIGHLOW / RVCONE / CALPB | Accepted | 2026-04-15 |
| 129 | Quant Stats Round 22 — RETSKEW / RETKURT / TAILR / RUNLEN / DAYRANGE | Accepted | 2026-04-15 |
| 131 | Quant Stats Round 23 — AUTOCOR / HURST / HITRATE / GLASYM / VOLRATIO | Accepted | 2026-04-15 |
| 132 | Quant Stats Round 24 — DRAWUP / GAPSTATS / VOLCLUSTER / CLOSEPLC / MRHL | Accepted | 2026-04-15 |
| 133 | Quant Stats Round 25 — DOWNVOL / SHARPR / EFFRATIO / WICKBIAS / VOLOFVOL | Accepted | 2026-04-15 |
| 134 | Quant Stats Round 26 — CALMAR / ULCER / VARRATIO / AMIHUD / JBNORM | Accepted | 2026-04-15 |
| 135 | Quant Stats Round 27 — OMEGA / DFA / BURKE / MONTHSEAS / ROLLSPRD | Accepted | 2026-04-15 |
| 136 | Quant Stats Round 28 — PARKINSON / GKVOL / RSVOL / CVAR / DOWEFFECT | Accepted | 2026-04-15 |
| 137 | Quant Stats Round 29 — STERLING / KELLYF / LJUNGB / RUNSTEST / ZERORET | Accepted | 2026-04-15 |
| 138 | Quant Stats Round 30 — PSR / ADF / MNKENDALL / BIPOWER / DDDUR | Accepted | 2026-04-15 |
| 139 | Quant Stats Round 31 — HILLTAIL / ARCHLM / PAINRATIO / CUSUM / CFVAR | Accepted | 2026-04-15 |
| 140 | Quant Stats Round 32 — ENTROPY / RACHEV / GPR / PACF / APEN | Accepted | 2026-04-16 |
| 141 | Quant Stats Round 33 — UPR / LEVEREFF / DRAWDAR / VARHALF / GINI | Accepted | 2026-04-16 |
| 142 | Quant Stats Round 34 — SAMPEN / PERMEN / RECFACT / KPSS / SPECENT | Accepted | 2026-04-16 |
| 143 | Quant Stats Round 35 — ROBVOL / RENYIENT / RETQUANT / MSENT / EWMAVOL | Accepted | 2026-04-16 |
| 144 | Quant Stats Round 36 — KSNORM / ADTEST / LMOM / KYLELAM / PEAKOVER | Accepted | 2026-04-16 |
| 145 | Quant Stats Round 37 — HIGUCHI / PICKANDS / KAPPA3 / LYAPUNOV / RANKAC | Accepted | 2026-04-16 |
| 146 | Quant Stats Round 38 — BNSJUMP / PPROOT / MFDFA / HILLKS / TSI | Accepted | 2026-04-16 |
| 147 | Quant Stats Round 39 — GARCH11 / SADF / CORDIM / SKSPEC / AUTOMI | Accepted | 2026-04-16 |
| 149 | Quant Stats Round 40 — DURBINWATSON / BDSTEST / BREUSCHPAGAN / TURNPTS / PERIODOGRAM | Accepted | 2026-04-16 |
| 150 | Quant Stats Round 41 — MCLEODLI / OUFIT / GPH / BURGSPEC / KENDALLTAU | Accepted | 2026-04-17 |
| 151 | TA-Lib + Godel Parity Round 42 — SQUEEZE / SQUEEZERANK / BBSQUEEZE / DONCHIAN / KAMA | Accepted | 2026-04-17 |
| 152 | TA-Lib + Godel Parity Round 43 — ICHIMOKU / SUPERTREND / KELTNER / FISHER / AROON | Accepted | 2026-04-17 |
| 153 | TA-Lib + Godel Parity Round 44 — ADX / CCI / CMF / MFI / PSAR | Accepted | 2026-04-17 |
| 154 | TA-Lib + Godel Parity Round 45 — VORTEX / CHOP / OBV / TRIX / HMA | Accepted | 2026-04-17 |
| 155 | TA-Lib + Godel Parity Round 46 — PPO / DPO / KST / ULTOSC / WILLR | Accepted | 2026-04-17 |
| 156 | TA-Lib + Godel Parity Round 47 — MASS / CHAIKOSC / KLINGER / STOCHRSI / AWESOME | Accepted | 2026-04-17 |
| 158 | Godel Parity Round 48 — EFI / EMV / NVI / PVI / COPPOCK | Accepted | 2026-04-17 |
| 159 | TA-Lib + Godel Parity Round 49 — CMO / QSTICK / DISPARITY / BOP / SCHAFF | Accepted | 2026-04-17 |
| 160 | TA-Lib + Godel Parity Round 50 — STOCH / MACD / VWAP / MCGD / RWI | Accepted | 2026-04-17 |
| 161 | TA-Lib + Godel Parity Round 51 — DEMA / TEMA / LINREG / PIVOTS / HEIKIN | Accepted | 2026-04-17 |
| 163 | TA-Lib + Godel Parity Round 52 — ALMA / ZLEMA / ELDERRAY / TSF / RVI | Accepted | 2026-04-17 |
| 164 | TA-Lib + Godel Parity Round 53 — TRIMA / T3 / VIDYA / SMI / PVT | Accepted | 2026-04-17 |
| 165 | Godel Parity Round 54 — AC / CHVOL / BBWIDTH / ELDERIMP / RMI | Accepted | 2026-04-17 |
| 167 | Godel Parity Round 55 — SMMA / ALLIGATOR / CRSI / SEB / IMI | Accepted | 2026-04-17 |
| 168 | TA-Lib + Godel Parity Round 56 — GMMA / MAENV / ADL / VHF / VROC | Accepted | 2026-04-17 |
| 169 | Godel Parity Round 57 — KDJ / QQE / PMO / CFO / TMF | Accepted | 2026-04-17 |
| 170 | TA-Lib + Godel Parity Round 58 — FRACTALS / IFT_RSI / MAMA / COG / DIDI | Accepted | 2026-04-17 |
| 171 | TA-Lib + Godel Parity Round 59 — DEMARKER / GATOR / BW_MFI / VWMA / STDDEV | Accepted | 2026-04-17 |
| 172 | TA-Lib + Godel Parity Round 60 — WMA / RAINBOW / MESA_SINE / FRAMA / IBS | Accepted | 2026-04-17 |
| 173 | TA-Lib + Godel Parity Round 61 — LAGUERRE_RSI / ZIGZAG / PGO / HT_TRENDLINE / MIDPOINT | Accepted | 2026-04-17 |
| 174 | TA-Lib + Godel Parity Round 62 — MASSINDEX / NATR / TTM_SQUEEZE / FORCE_INDEX / TRANGE | Accepted | 2026-04-17 |
| 175 | TA-Lib + Godel Parity Round 63 — LINEARREG_SLOPE / HT_DCPERIOD / HT_TRENDMODE / ACCBANDS / STOCHF | Accepted | 2026-04-17 |
| 176 | TA-Lib Parity Round 64 — LINEARREG / LINEARREG_ANGLE / HT_DCPHASE / HT_SINE / HT_PHASOR | Accepted | 2026-04-17 |
| 177 | TA-Lib Parity Round 65 — MIDPRICE / APO / MOM / SAREXT / ADXR | Accepted | 2026-04-18 |
| 178 | TA-Lib Parity Round 66 — AVGPRICE / MEDPRICE / TYPPRICE / WCLPRICE / VARIANCE | Accepted | 2026-04-18 |
| 179 | TA-Lib Parity Round 67 — PLUS_DI / MINUS_DI / PLUS_DM / MINUS_DM / DX | Accepted | 2026-04-18 |
| 180 | TA-Lib Parity Round 68 — ROC / ROCP / ROCR / ROCR100 / CORREL | Accepted | 2026-04-18 |
| 181 | TA-Lib Parity Round 69 — MIN / MAX / MINMAX / MININDEX / MAXINDEX | Accepted | 2026-04-18 |
| 182 | TA-Lib Parity Round 70 — BBANDS / AD / ADOSC / SUM / LINEARREG_INTERCEPT | Accepted | 2026-04-18 |
| 183 | TA-Lib Parity Round 71 — AROONOSC / MINMAXINDEX / MACDEXT / MACDFIX / MAVP | Accepted | 2026-04-18 |
| 184 | TA-Lib Parity Round 72 — CDLDOJI / CDLHAMMER / CDLSHOOTINGSTAR / CDLENGULFING / CDLHARAMI | Accepted | 2026-04-18 |
| 185 | TA-Lib Parity Round 73 — CDLMORNINGSTAR / CDLEVENINGSTAR / CDL3BLACKCROWS / CDL3WHITESOLDIERS / CDLDARKCLOUDCOVER | Accepted | 2026-04-18 |
| 186 | TA-Lib Parity Round 74 — CDLPIERCING / CDLDRAGONFLYDOJI / CDLGRAVESTONEDOJI / CDLHANGINGMAN / CDLINVERTEDHAMMER | Accepted | 2026-04-19 |
| 187 | TA-Lib Parity Round 75 — CDLHARAMICROSS / CDLLONGLEGGEDDOJI / CDLMARUBOZU / CDLSPINNINGTOP / CDLTRISTAR (research-layer, chart overlay deferred) | Implemented (research layer); chart overlays governed by ADR-079 | 2026-04-19 |
| 188 | Chart-Drawing Parity Deferred — Research-Packet-First | Accepted | 2026-04-19 |
| 189 | Parity Expansion R76-R78 — Quant Stats + TA-Lib Research Surfaces | Accepted | 2026-04-20 |
| 190 | Parity Expansion R79-R80 — Additional TA-Lib Candlestick Research Surfaces | Accepted | 2026-04-20 |
| 191 | Parity Expansion R81-R82 — Harder TA-Lib Candlestick Research Surfaces | Accepted | 2026-04-20 |
| 192 | Parity Expansion R83-R84 — Additional Multi-Bar TA-Lib Candlestick Research Surfaces | Accepted | 2026-04-20 |
| 193 | Parity Expansion R85-R86 — Stateful TA-Lib Candlestick Research Surfaces | Accepted | 2026-04-21 |
| 194 | Parity Expansion R87-R88 — Final TA-Lib Candlestick Research Surfaces | Accepted | 2026-04-21 |
| 195 | Parity Expansion R89-R90 — Deferred Benchmark and Peer-Relative Momentum Surfaces | Accepted | 2026-04-21 |
| 196 | Parity Expansion R91-R92 — Deferred Sector Liquidity and Benchmark-Link Ranks | Accepted | 2026-04-21 |
| 197 | Parity Expansion R93/R94 — OPERANK_DELTA / DIVACC / EPSACC / VRP | Accepted | 2026-04-21 |
| 198 | Parity Expansion R95 — SHORTRANK_DELTA + Short-Interest History | Accepted | 2026-04-21 |
| 199 | Parity Expansion R96 — INSIDERCONC | Accepted | 2026-04-22 |
| 200 | Chart Parity R97 — GPU/CPU Chart Indicators For CMO, QSTICK, DISPARITY, BOP, STDDEV | Accepted | 2026-04-22 |

## Maintenance rule

If a future parity feature changes data ownership, cache layout, broker API contracts, or chart-overlay architecture, write a focused ADR for that architectural decision. If it merely adds another research packet, scanner column, TA-Lib primitive, or egui view under the same architecture, update this ADR and the user-facing docs instead.
