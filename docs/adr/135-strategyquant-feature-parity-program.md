# ADR-135: StrategyQuant X + NNFX Algo Tester Feature-Parity Program — Correctness First

**Status:** Accepted (roadmap — direction is binding, milestones are gated, not dated)
**Date:** 2026-07-27
**Reference-product evidence date:** 2026-07-27 (official vendor pages; see [Primary sources](#primary-sources))
**Scope owner:** `typhoon-engine` simulation/search core + `typhoon-native` strategy GUI

---

## 1. Context

### 1.1 What TyphooN has today — a first-draft foundation, not a research platform

The current backtester and optimizer are a **working first draft**. They demonstrate the
shape of the product (define a strategy → run it → rank parameters → look at a heatmap →
check walk-forward), and they are genuinely useful as an interactive chart companion. They
are **not** a system whose numbers should be trusted for capital-allocation decisions, and
this ADR states that plainly so that future work does not inherit a false baseline.

**Engine — `typhoon-engine/src/core/backtest.rs` (1,183 lines, 53 unit tests in
`backtest/tests.rs`):**

| Element | Present state |
| --- | --- |
| Strategy interface | `trait Strategy { fn on_bar(&mut self, bar, index, bars) -> Option<Signal>; fn name(&self) }` — one signal per closed bar |
| Strategy inventory | Five hand-written Rust strategies: `SMACrossStrategy`, `NNFXStrategy`, `KAMACrossStrategy`, `FisherCrossStrategy`, `RSIMeanRevStrategy` |
| Signals | `Signal::{Buy, Sell, Close}` — no order type, size, price, or validity concept |
| Execution | `run_backtest` — always-in-market reversal model; every fill is at **the same bar's close** that produced the signal |
| Position sizing | Fixed notional `initial_equity / entry_price` units; no compounding, no risk model, no margin |
| Costs | **None.** No commission, no spread, no slippage, no funding, no borrow |
| Equity curve | Realized-PnL only (equity moves when a trade closes); no mark-to-market |
| Metrics | `TradeReport`: trades, win rate, profit factor, Sharpe, max DD (abs + %), max consecutive win/loss, avg win/loss/trade, total PnL, gross profit/loss |
| Optimization | `optimize_sma_cross` — nested `for` grid over SMA fast × slow only, ranked by profit factor, truncated to top-N |
| Walk-forward | `walk_forward` — `num_windows` equal slices, hard-coded **70 % IS / 30 % OOS**, SMA-only, IS-Sharpe selection, `robustness_score = oos_sharpe / is_sharpe` |
| Replay | `bar_by_bar_backtest` → `Vec<BarState>` for chart replay |

**Native GUI — `typhoon-native/src/app/strategy_windows.rs` (926 lines):**

- `render_backtest_window` — backtest of the **current chart symbol only**, single symbol,
  single timeframe.
- `render_optimizer_window` — CPU grid path (`backtest::optimize_sma_cross`) plus two GPU
  paths (`gpu_compute::ParamCombo` for SMA, `NnfxParamCombo` for NNFX), a top-50 result
  grid, a PnL plot, a **Fast × Slow → Sharpe** heatmap, and a walk-forward summary.

**GPU — `typhoon-native/src/gpu_compute`:** fixed WGSL shaders for the SMA and NNFX
parameter sweeps plus fixed result metrics. It is a **parameter-sweep accelerator for two
hard-coded strategies**. It is not a general strategy generator, not a general strategy
evaluator, and not an execution-realistic simulator.

### 1.2 Specific first-draft defects that must not be treated as a baseline

These are properties of the current code, verified by reading it, and each one is a
correctness requirement later in this ADR:

1. **Look-ahead is structurally possible.** `on_bar` receives the *entire* `bars: &[Bar]`
   slice on every call. Nothing prevents a strategy from reading `bars[index + 1..]`. Only
   convention keeps the current five honest. A trustworthy engine must make look-ahead
   *unrepresentable*, not merely discouraged.
2. **Zero-latency same-bar fills.** The signal is derived from a bar's close and filled at
   that same close. Real execution cannot see a close and trade at it.
3. **Zero cost model.** With no spread/commission/slippage, every high-frequency parameter
   region is systematically over-valued, and the optimizer's ranking is biased toward
   exactly the strategies that would die on contact with a live fee schedule.
4. **Profit-factor ranking is degenerate.** `profit_factor` is clamped to a `999.0`
   sentinel when there are no losing trades, and the grid sorts by PF descending — so a
   two-trade, zero-loss fluke outranks a robust 400-trade result. There is no minimum
   trade-count gate.
5. **Sharpe is not a defined statistic.** It is computed from **per-trade** `pnl_pct`
   values, with population (not sample) standard deviation, no risk-free rate, and a flat
   `√252` annualization regardless of the bar timeframe or the trades-per-year rate. Two
   runs on different timeframes produce Sharpe numbers that are not comparable.
6. **Drawdown is realized-only.** Max DD is walked over trade-close equity, so intra-trade
   and intrabar excursion is invisible. There is no MAE/MFE at all.
7. **Robustness score is a single ratio.** `oos_sharpe / is_sharpe` from one fixed 70/30
   split is a smoke test, not a robustness verdict; with a near-zero denominator it is not
   even numerically meaningful.
8. **Single symbol, single timeframe, single strategy.** No portfolio, no multi-market, no
   multi-timeframe, no cross-strategy correlation, no shared capital.

None of this is a criticism of the draft — it is the correct amount of machinery for a
first pass. It is recorded here so that Milestone 1 is understood as *replacing* the
simulation core, not decorating it.

### 1.3 Why an ADR now

The strategy tooling is the one area of TyphooN where the gap to mature commercial products
is both large and *legible*. StrategyQuant X publishes a broad research-platform feature
list, while NNFX Algo Tester documents a narrower guided construction, replay, and risk
workflow. Together they form a useful external checklist of capabilities that retail quant
users demonstrably want. Without a durable decision record, this area will keep
accreting one-off windows (a heatmap here, a shader there) that cannot compose into a
research workflow, and [ADR-038](038-gpu-strategy-optimizer-and-mql5-export-pipeline.md)
will keep being read as an accurate status report when parts of it are stale.

### 1.4 What the reference products publicly claim

Everything below is **advertised capability from the vendors' own public marketing and help
pages**, captured **2026-07-27**. It is recorded as *their public claim*, not as independently
verified behaviour. This ADR makes **no assertion whatsoever about proprietary algorithms,
data structures, or implementation** — those are not public, are not needed, and are not to
be guessed at in this repository.

**StrategyQuant X — features page:** machine-learning strategy generation; no-code
AlgoWizard strategy building; any market and timeframe including multi-market and
multi-timeframe; real-tick backtesting; portfolios; strategy improver; retester/optimizer;
chart trade display; multiple out-of-sample periods; MAE/MFE and daily-equity analysis;
fit-to-portfolio; problem recognition; custom indicators and models; fully automatic custom
workflows; templates and random placeholders; fuzzy logic; scaling out; custom
analysis/plugins/scripts.

**StrategyQuant X — robustness section of the same page:** integrated cross-checks; two
Monte Carlo families spanning 9+ simulation types; walk-forward analysis, walk-forward
optimization and a walk-forward matrix; 3D optimization views; System Parameter
Permutation; Optimization Profile; advanced filtering.

**QuantAnalyzer:** detailed reports; What-If analysis; money-management simulation; Monte
Carlo; equity-control simulation; portfolio construction and comparison; extensibility.

**QuantDataManager:** multi-source tick / minute / EOD data; import; gap, spike and
bad-candle QA; chart and table inspection; timeframe and timezone transforms; verified
downloads.

**NNFX Algo Tester — official feature list and help:** no-code indicator-based strategy
configuration; automatic and manual/hybrid backtesting; visual replay; automatic forward
testing/live trading and a trade assistant; Full Algo, C1-signal, baseline-signal, and
baseline/C1-signal profiles; dedicated repainting-indicator and weekend-candle diagnostics;
Open Prices Only and Every Tick operation models; configurable decision time before candle
close; NNFX entry/rule toggles; 75+ bundled indicators plus custom indicators; nine typed
indicator roles; multi-timeframe and external-market/news inputs; parameter optimization with
up to 25 inputs; Candidate Search across indicator choices/combinations; long-only/short-only
modes; detailed customizable reports and CSV/Excel export.

Its documented trade-management surface includes two-leg entries; ATR-derived or fixed
SL/TP; break-even and stepped trailing-stop rules; fixed-size or fixed-risk sizing; virtual
targets/FIFO-compatible operation; and account-wide currency overexposure policies that can
block or reduce risk for same-currency or same-direction entries.

### 1.5 Why both references matter

The products cover different halves of the desired system:

- **StrategyQuant X is the breadth benchmark** for generation, retesting, robustness,
  experiment databanks, portfolio construction, data management, and automated research
  workflows.
- **NNFX Algo Tester is the guided-workflow benchmark** for assembling indicator-role
  strategies, isolating individual signal components, mixing automatic and manual replay,
  carrying one configuration from backtest into paper/live operation, and applying practical
  NNFX trade/risk rules.

TyphooN should combine those user capabilities around one deterministic Rust engine. It
should not reproduce either vendor's UI, MT4 packaging, binary indicator format, or internal
implementation.

---

## 2. Decision

**Adopt a staged StrategyQuant-X-class research program plus NNFX-Algo-Tester-class guided
strategy workflow for TyphooN, with simulation correctness as the non-negotiable first
milestone.**

Concretely:

1. **Label the current backtester/optimizer as a first-draft foundation** in docs and in
   the GUI, and stop citing its metrics as decision-grade. (§1.1–§1.2 is that label.)
2. **Define parity as user-capability/workflow parity** — the things a user can *do* and
   *conclude* across both reference products — never as UI cloning, algorithm cloning, or
   service cloning (§3).
3. **Rebuild around a layered architecture** (§5) whose spine is a versioned strategy IR,
   a deterministic event-driven simulator, immutable datasets with provenance, and an
   experiment databank.
4. **Treat execution realism (§6) and metric definition (§9) as normative requirements**,
   with golden tests, before any broad generation/optimization work is expanded.
5. **Gate every milestone on acceptance criteria, not on dates** (§13).
6. **Supersede the program-level scope of [ADR-038](038-gpu-strategy-optimizer-and-mql5-export-pipeline.md)**
   and correct its stale claims without deleting it (§15).

---

## 3. What "parity" means here

**Parity means:** a TyphooN user can carry out the same useful *research and guided strategy
workflows* and reach the same *classes of conclusion* as users of the two reference products
— assemble an NNFX-style indicator system without coding, inspect or intervene in its replay,
carry it into paper/live validation, build or generate broader strategies, simulate them with
realistic execution, stress them for robustness, keep the survivors in a searchable databank,
assemble them into a portfolio, and re-validate them over time.

**Parity explicitly does NOT mean:**

| Non-target | Reason |
| --- | --- |
| Cloning either product's UI, layout, window set, or visual identity | TyphooN is a native egui terminal ([ADR-115](115-deprecate-cli-tui.md), [ADR-125](125-native-crate-boundary-plan.md)); our GUI conventions win |
| Reproducing proprietary algorithms | Not public, not inferable, not to be reverse-engineered or guessed |
| A cloud/SaaS tier, licensing server, or marketplace | TyphooN is a local-first single-user desktop app |
| Their broker/platform matrix or code-export targets (MT4/MT5/cTrader/etc.) | MT5 scope was removed by [ADR-111](111-broker-scope-reduction-kraken-alpaca-only.md); active scope is **Kraken + Alpaca** |
| Paid tick-data vendor integrations | Data scope stays free/entitled sources already wired ([ADR-112](112-equities-bar-sync-demand-depth-vs-catalog-breadth.md), [ADR-128](128-sync-coverage-levers.md)) |
| Feature-count matching for its own sake | A capability we cannot make *correct* is worse than one we do not ship |

**Parity is measured by workflow acceptance tests**, not by checklist length: *"can a user
generate 10k candidate strategies on BTC/USD 1h, reject the ones that fail cost-adjusted
OOS, and export a ranked, reproducible databank with full provenance?"* — that is a parity
question. *"Do we have a button in the same place?"* — is not.

---

## 4. Feature taxonomy & parity matrix

Status vocabulary (used consistently in this document):

- **Current** — implemented, works, adequate for its stated purpose.
- **Foundation** — code exists but is a first-draft scaffold; demonstrates shape, not
  trustworthy for capital decisions.
- **Partial** — a real, usable subset exists with named capability gaps.
- **Missing** — not present in any form.
- **Out-of-scope** — deliberately not pursued; reason given.

Left column = capability publicly advertised by either reference product (evidence date 2026-07-27).
Right columns = TyphooN's honest present state and where it is addressed in this ADR.

### 4.1 Strategy construction & generation

| Capability (vendor public claim) | TyphooN status | Addressed in |
| --- | --- | --- |
| Machine-learning / automated strategy generation | **Missing** | §8, M5 |
| No-code visual strategy builder (AlgoWizard-class) | **Missing** — strategies are hand-written Rust structs | §5.2, M3 |
| Custom indicators & models | **Partial** — a large native indicator library plus `typhoon-transpiler` ([ADR-040](040-typhoon-transpiler-pipeline-source-to-gpu-cpu-execution.md), [ADR-067](067-multi-frontend-expansion-cross-language-transpiler.md)), but no strategy-level plug-in surface | §5.10, M6 |
| Templates / random placeholders in generated logic | **Missing** | §8.1, M5 |
| Fuzzy logic building blocks | **Missing** | §8.1 (deferred), M5+ |
| Strategy improver (iterative refinement of an existing strategy) | **Missing** | §8.4, M6 |
| Scaling out / partial exits | **Foundation** — canonical two-leg management lowers to independent reduce-only target/stop OCO groups with break-even, trailing and time-stop handling; native authoring/results UX remains open | §6.7, M2 |

### 4.2 Simulation & execution realism

| Capability | TyphooN status | Addressed in |
| --- | --- | --- |
| Bar-close backtesting | **Current kernel** — realistic next-open execution is the default; the old same-close result is isolated as an explicitly named compatibility bridge | §6, M1 |
| Real-tick backtesting | **Blocked** — the engine deliberately has no `Tick` fidelity variant because TyphooN retains no versioned tick history; `SubBar` is the highest honest level until such a corpus exists | §6.9, M2/M7 (data-gated) |
| Intrabar modelling / OHLC ambiguity policy | **Current kernel** — pessimistic stop-first, target-first and documented OHLC-path policies, with gap-through-open handling | §6.1, M1 |
| Bid/ask & spread modelling | **Foundation** — constant/percentage spread models execute against side-correct quotes; recorded quote input remains unsupported | §6.2, M1 |
| Commissions / fees / funding / borrow | **Foundation** — per-fill commissions plus identity-bearing constant financing/borrow/funding assumptions and accrual events; unavailable borrow/live rate inputs fail the run instead of becoming zero | §6.3, M1/M2 |
| Slippage & latency | **Foundation** — fixed/spread-fraction slippage and fixed/seeded-uniform two-leg latency are deterministic and run-stamped; volatility-scaled slippage remains unsupported | §6.4, M1 |
| Order types beyond immediate market reversal | **Foundation** — market, limit, stop, stop-limit, market-on-close, IOC/FOK/Day/GTC/GTD, reduce-only and OCO lifecycles execute in the kernel | §6.5, M1 |
| Partial fills / liquidity caps | **Foundation** — a run-stamped bar-volume participation cap is shared across orders and sub-bars; remainders rest or expire by TIF, including IOC/FOK and OCO resizing. L2 depth-aware fills remain open | §6.6, M2 |
| Sessions & timezones | **Foundation** — per-instrument UTC/US-Eastern calendars gate every execution and queue or reject closed-session submissions with DST-correct local windows and rule-based US half days. Published closures, early closes and open overrides now load from content-addressed exception artifacts sealed from persisted source records; acquiring exchange/vendor source feeds remains open | §6.7, M2 |
| Corporate actions in simulation | **Foundation** — split, dividend, symbol-change and delisting events adjust live state at effective time; verified run assembly rejects events already represented by split-adjusted/total-return prices. Schedules now materialize from content-addressed source artifacts; cash-in-lieu and the automated fetchers that would persist those source records remain open | §6.8, M2 |
| Multi-market / multi-symbol simultaneous simulation | **Foundation** — the reference kernel has a deterministic global multi-symbol event clock; portfolio/shared-capital policy and native orchestration remain M4 work | §6.11, M1/M4 |
| Multi-timeframe within one strategy | **Missing** in the simulator (MTF chart overlay exists — [ADR-123](123-mtf-overlay-price-scale-consistency.md)) | §6.11, M1/M4 |
| Deterministic, reproducible runs | **Current kernel** — sealed run manifests, derived seeded streams, stable event ordering and concurrent bit-identity are gate-tested | §6.10, M1 |
| No-look-ahead guarantee | **Current kernel** — committed-bar views, forming-bar restrictions and compile/runtime canaries are M1-gate tested | §6.12, M1 |

### 4.3 Optimization & search

| Capability | TyphooN status | Addressed in |
| --- | --- | --- |
| Parameter optimizer over arbitrary strategies | **Foundation** — SMA-only grid; GPU path is SMA/NNFX-only | §5.5, M4 |
| Retester (re-run stored strategies under new settings/data) | **Missing** | §5.5, M4 |
| Genetic / evolutionary search | **Missing** | §8.2, M5 |
| Random / grid / Bayesian / local search | **Partial** — grid only | §8.2, M4/M5 |
| 3D optimization views | **Partial** — 2D Fast × Slow heatmap only | §5.11, M4 |
| Optimization Profile (stability across the parameter field) | **Missing** | §7.4, M4 |
| Advanced result filtering | **Partial** — top-N truncation by profit factor | §5.7, M3 |

### 4.4 Robustness & validation

| Capability | TyphooN status | Addressed in |
| --- | --- | --- |
| Walk-forward analysis | **Foundation** — fixed 70/30, SMA-only, single ratio score | §7.2, M4 |
| Walk-forward optimization & matrix | **Missing** | §7.2, M4 |
| Multiple out-of-sample periods | **Missing** — one split | §7.1, M4 |
| Monte Carlo (multiple simulation families/types) | **Missing for strategy robustness** — the wired GPU routine resamples historical daily returns for forward portfolio VaR; it is not trade/data/execution perturbation | §7.3, M4 |
| System Parameter Permutation | **Missing** | §7.4, M4 |
| Parameter-plateau / neighbour-stability analysis | **Foundation only** — a robustness pipeline/shader is constructed but has no host dispatch API and is not part of a gated workflow | §7.4, M4 |
| Cross-checks across market / timeframe / data source | **Missing** | §7.5, M4 |
| Problem recognition (automatic red-flag detection) | **Missing** | §7.6, M4 |
| Multiple-testing / selection-bias controls | **Missing** — not represented anywhere | §7.7, M4 |
| Holdout / quarantine discipline | **Missing** | §7.8, M4 |

### 4.5 Analysis, reporting & portfolio

| Capability | TyphooN status | Addressed in |
| --- | --- | --- |
| Detailed performance reports | **Foundation** — 14 metrics, no confidence intervals, ambiguous Sharpe | §9, M2 |
| MAE / MFE analysis | **Missing** | §9.2, M2 |
| Daily-equity analysis | **Missing** — equity is per-trade-close, not calendar | §9.2, M2 |
| Chart trade display | **Partial** — `bar_by_bar_backtest` replay exists; no annotated trade overlay | §5.11, M3 |
| What-If analysis | **Missing** | §10.4, M6 |
| Money-management simulation | **Missing** — fixed notional only | §10.3, M6 |
| Equity-control simulation | **Missing** | §10.4, M6 |
| Portfolio construction / comparison | **Missing** | §10, M6 |
| Fit-to-portfolio objective | **Missing** | §10.2, M6 |
| Correlation-aware strategy selection | **Missing** | §10.1, M6 |
| Databank of stored strategies with metadata | **Missing** — results are in-memory, lost on window close | §5.7, M3 |

### 4.6 Data management & QA

| Capability | TyphooN status | Addressed in |
| --- | --- | --- |
| Multi-source data ingest | **Current** — Kraken + Alpaca + Yahoo merge ([ADR-112](112-equities-bar-sync-demand-depth-vs-catalog-breadth.md), [ADR-113](113-cross-source-equity-bar-merge-data-integrity.md)) | — |
| Tick data | **Missing** for history; L1/L2 live only ([ADR-129](129-l1-l2-l3-market-data-support.md)) | §11.3 (data-gated) |
| Minute & EOD data | **Current** | — |
| Gap / spike / bad-candle QA | **Current** — deterministic dataset QA pass with a versioned policy, stored with the dataset and sealed into its manifest (§11.1, M0) | §11.2, M0 |
| Chart & table inspection of raw data | **Current** — chart plus the Dataset Inspector's bounded, QA-flagged bar table (M0) | §11.2, M0 |
| Timeframe transforms / resampling | **Partial** — derivation exists in sync/merge; not a user-driven dataset tool | §11.1, M0 |
| Timezone transforms | **Missing** as a user control | §6.7, M2 |
| Verified / integrity-checked downloads | **Partial** — materialized datasets carry a content-addressed manifest, a sealed QA report, and a digest-verified payload (M0); the upstream fetches themselves are still unverified at download time | §11.1, M0 |
| Custom data import | **Missing** | §11.4 (deferred) |

### 4.7 Workflow & extensibility

| Capability | TyphooN status | Addressed in |
| --- | --- | --- |
| Fully automatic custom workflows (build → test → filter → store) | **Missing** — every step is a manual button | §5.9, M7 |
| Custom analysis / plugins / scripts | **Missing** at the strategy layer | §5.10, M6 |
| Scheduled / unattended runs | **Missing** | §5.9, M7 |
| Code export to external trading platforms | **Out-of-scope** — MT5/export removed by [ADR-111](111-broker-scope-reduction-kraken-alpaca-only.md); TyphooN executes on **Kraken + Alpaca** natively ([ADR-126](126-primary-assist-broker-selection.md)) | §14 |
| Cloud/distributed compute | **Out-of-scope** — local-first | §14 |

### 4.8 NNFX-guided construction, testing, and operation

This matrix captures the guided indicator-system workflow advertised by NNFX Algo Tester.
TyphooN targets these capabilities natively; MT4-specific packaging is not the target.

| Capability (NNFX Algo Tester public claim) | TyphooN status | Addressed in |
| --- | --- | --- |
| Guided indicator-role builder (ATR, baseline, C1, C2, volume, exit, continuation, news/market filters) | **Missing as a builder** — a fixed NNFX Rust strategy and broad indicator library exist, but no role-based builder or saved configuration | §5.2, §5.11, M3 |
| Full-algorithm, C1-only, baseline-only, and baseline+C1 test profiles | **Missing** | §5.2, M3 |
| NNFX baseline/standard/continuation/pullback entries and One Candle/A Bridge Too Far rule toggles | **Missing as configurable IR** — only fixed NNFX logic exists | §5.2, M3 |
| Automatic plus manual/hybrid backtesting | **Missing** — simulation cannot pause for a recorded user decision | §6.13, M2 |
| Visual mode with entries, exits, stop updates, and indicator state | **Missing as a visual workflow** — bar-state replay data is a useful engine foundation, but no annotated replay UI is wired | §5.11, M2/M3 |
| Repainting-indicator diagnostic | **Missing** | §11.5, M2 |
| Weekend-candle diagnostic | **Current** — session-aware dataset QA under a versioned per-instrument calendar policy; crypto weekend bars stay valid, equity/xStock bars are judged against their own venue rule (M0) | §11.5, M0 |
| Open-price and tick operation models | **Missing** — the draft uses same-bar close fills; the target fidelity ladder is broader and explicit | §6.9, M1/M2/M7 |
| Configurable decision time before candle close | **Missing** | §6.13, M2 |
| 75+ bundled indicators and custom indicator loading | **Partial** — TyphooN has 46+ chart indicators and a transpiler direction; loading MT4 `.ex4` binaries is out-of-scope | §5.10, §14, M6 |
| Multi-timeframe indicator evaluation and external market/news filters | **Partial outside simulation** — chart MTF/news/market data exist; strategy-time synchronization does not | §6.11, M1/M4 |
| Up to 25 optimization inputs | **Foundation** — fixed SMA/NNFX parameter sweeps only; TyphooN will not impose an arbitrary 25-input ceiling | §5.5, M4 |
| Candidate Search across indicators and slot combinations | **Missing** — maps to typed holes/templates and constrained generation | §8.1–§8.3, M5 |
| Long-only / short-only testing | **Missing as a run constraint** | §5.5, M3 |
| Two-leg ATR/fixed SL/TP, break-even, stepped trailing stop, fixed-risk/fixed-size sizing, and virtual targets | **Partial outside backtesting** — TyphooN's risk/order code has useful foundations, but the simulator does not model this lifecycle | §6.5–§6.7, §10.3, M2/M6 |
| Currency/asset-group overexposure blocking or risk reduction | **Missing** | §10.3, M6 |
| Same saved configuration for backtest → paper/forward → live assistant/automation | **Missing end-to-end** — broker execution exists, but no validated strategy promotion pipeline | §10.5, M8 |
| Strategy signal notifications and manual/automatic execution choice | **Partial infrastructure** — alerts and broker controls exist, but are not driven by a persisted validated strategy | §10.5, M8 |
| Detailed customizable reports with balance/equity charts and CSV/Excel export | **Foundation** — summary metrics and equity display exist; versioned report/export contracts do not | §9, M2 |

---

## 5. Target architecture

Eleven layers. Each is separately testable; each higher layer depends only on the contracts
below it. The spine is: **IR → dataset → simulator → metrics → databank**. Search,
robustness, portfolio and workflow all consume that spine rather than re-implementing it.

```
 L11  Native GUI (egui)  ── builder · runner · databank browser · portfolio · workflow
 L10  Extensions         ── indicator/metric/objective/filter plug-ins (Rust + transpiler)
  L9  Workflow DAG       ── typed jobs, scheduling, resume, artifact cache
  L8  Portfolio & MM     ── correlation, capital/margin, money management, what-if
  L7  Databank           ── immutable runs, strategy identity, provenance, query
  L6  Robustness         ── OOS schemes, WF matrix, MC families, SPP, cross-checks, gates
  L5  Search             ── grid · random · Bayesian · local · evolutionary generation
  L4  Metrics            ── versioned, fully-defined statistic set + uncertainty
  L3  Simulator          ── deterministic event-driven execution & accounting kernel
  L2  Strategy IR        ── versioned typed AST + compilers (interp · JIT-ish · WGSL)
  L1  Datasets           ── immutable, content-addressed, QA'd, provenance-carrying
  L0  Bar cache / sync   ── existing SQLite + merge stack (ADR-003, 112, 113)
```

### 5.1 L1 — Immutable datasets with provenance

A **Dataset** is an immutable, content-addressed snapshot of market data for a
(symbol, timeframe, time-range, source-policy) tuple, materialised out of the existing
cache ([ADR-003](003-sqlite-bar-cache.md)) rather than read live.

- Identified by a **content hash** over the bar payload plus a **manifest**: symbol, venue,
  timeframe, UTC range, source/merge policy, adjustment policy, split/dividend table
  version, QA report hash, engine schema version.
- **Never mutated.** A resync produces a *new* dataset id. A run pinned to dataset `D` is
  reproducible forever, or fails loudly if `D` was garbage-collected.
- Carries the **merge lineage** ([ADR-113](113-cross-source-equity-bar-merge-data-integrity.md),
  [ADR-124](124-depth-era-promotion-must-not-redefine-price-scale.md)) so a result can be
  attributed to a data decision, not just to a strategy.
- Adjustment policy is **explicit and recorded** (raw vs split-adjusted vs total-return).
  A run may not silently mix policies across symbols.

### 5.2 L2 — Canonical versioned strategy IR/AST + visual builder

The single most important structural change: **a strategy stops being a Rust `impl` and
becomes data.**

- **Typed AST.** Nodes are typed (`Price`, `Series<f64>`, `Bool`, `Duration`, `Qty`,
  `Money`, `Percent`, `SessionTime`); type-checked at construction. Illegal compositions
  (comparing a `Money` to a `Percent`, feeding a `Bool` where a `Series` is required) are
  rejected before any simulation runs.
- **Explicit temporal semantics.** Every series access carries a non-negative `bars_ago`
  value (`0` = the latest observation visible at the current decision event); future
  observations are **not representable**. This is how §6.12's no-look-ahead guarantee is
  enforced — by the grammar, not by reviewer vigilance.
- **Versioned schema** (`ir_version`), with a migration path. A stored strategy from an old
  version is either migrated deterministically or marked unrunnable — never
  silently reinterpreted.
- **Canonical form + stable identity.** A normalisation pass (constant folding, commutative
  ordering, dead-branch removal) yields a canonical serialization whose hash is the
  **strategy id**. Two structurally identical strategies discovered by different search runs
  get the same id — this is what makes dedup (§8.3) and the databank (§5.7) work.
- **Multiple back-ends from one IR:** (a) a reference tree-walking interpreter — the
  correctness oracle; (b) a compiled/flattened evaluator for CPU throughput; (c) a WGSL
  emitter for GPU sweeps. **The interpreter is definitional**; the others must match it
  within a declared tolerance or they are bugs (§12.3).
- **Visual builder (GUI)** edits the IR directly — node palette, typed sockets, live
  validation, and a plain-text canonical view. The transpiler stack
  ([ADR-040](040-typhoon-transpiler-pipeline-source-to-gpu-cpu-execution.md),
  [ADR-067](067-multi-frontend-expansion-cross-language-transpiler.md)) is a *front-end*
  that lowers source into the IR — it is not a second strategy representation.
- **Guided NNFX profile editor** is a constrained view over that same IR, not a parallel
  engine: named slots for ATR, baseline, C1/C2, volume, exit, continuation, news, and external
  market filters; Full Algo/C1/BL/BL+C1 profiles; explicit entry/rule toggles; long/short
  constraints; and two-leg trade-management templates. Switching to the general node editor
  reveals the exact IR generated by the guided form.

### 5.3 L3 — Deterministic event-driven simulator

Replaces `run_backtest` wholesale.

- **Event loop, not a bar loop.** A single time-ordered queue of typed events: `BarOpen`,
  `Tick`/`Quote`, `BarClose`, `OrderSubmit`, `OrderActivate`, `Fill`, `PartialFill`,
  `Cancel`, `Expire`, `SessionOpen`, `SessionClose`, `CorporateAction`, `FundingCharge`,
  `MarginCall`, `Timer`. Deterministic tie-breaking by `(timestamp, priority, sequence)` —
  never by hash-map iteration order.
- **Strict separation of decision time and execution time.** Strategy code observes state
  as of event time `t` and may only emit orders that execute at `t + latency`.
- **User interventions are events.** Manual/hybrid replay never mutates hidden simulator
  state. Pause, submit/cancel/modify, skip-signal, and resume actions are timestamped
  `UserDecision` events stored in the run manifest, so the resulting run is replayable and
  auditable rather than an irreproducible click session.
- **Accounting kernel:** positions, average cost, realized/unrealized PnL, cash, margin,
  financing, fees, per-instrument contract specs. Mark-to-market on every bar, so equity is
  a real time series (fixing §1.2 item 6).
- **Portfolio-native from the start.** Even the single-symbol case runs through the
  multi-symbol clock (§6.11) so that portfolio support later is a configuration, not a
  rewrite.
- **Reference implementation is CPU, scalar, and readable.** Speed comes later and only
  under equivalence tests.

### 5.4 L4 — Metrics (see §9)

A versioned metric registry. Every metric has an id, a written definition, units, an
annualization rule, edge-case behaviour, and a golden test. `metrics_version` is stamped on
every run.

### 5.5 L5 — Search: builder / generator / optimizer / retester

One search interface over the IR:

- **Search space** = the IR with typed holes (parameter ranges, indicator slots, structural
  choices).
- **Strategies:** grid, random, Latin-hypercube, coordinate/pattern local search, Bayesian
  (TPE/GP over mixed spaces), evolutionary (§8.2). All share the same evaluation,
  seeding, checkpointing and dedup machinery.
- **Retester:** re-evaluate any stored strategy id against a *different* dataset, cost
  model, or metric version — the mechanism behind cross-checks (§7.5) and periodic
  re-validation (§10.5).
- **Objectives are first-class:** single- or multi-objective (NSGA-II-style Pareto fronts
  for e.g. return × drawdown × stability × trade count), with explicit complexity and
  trade-count penalties (§8.5). **Ranking by raw profit factor is retired** (§1.2 item 4).

### 5.6 L6 — Robustness pipeline (see §7)

A declarative pipeline of stages, each producing a verdict and evidence. Candidates flow
through cheap gates before expensive ones. Every rejection records *which* gate and *why*.

### 5.7 L7 — Databank / experiment store

- SQLite-backed (reusing the existing cache infrastructure and read-connection discipline —
  see the `try_connection` non-reentrancy constraint), holding: strategies (canonical IR +
  id), runs (strategy id × dataset id × cost model × engine version × metrics version +
  seed), metric vectors, robustness verdicts, and lineage edges (parent/child from
  evolutionary search, retest links).
- **Indexed for O(1)-ish query** on the fields the UI filters by — no full-table scan on the
  render thread ([ADR-098](098-per-frame-o-1-discipline-in-chart-and-sync-paths.md),
  [ADR-134](134-render-independent-background-pump.md)). Browsing is a bounded window over
  an indexed query, never a load-everything.
- **Runs are immutable and append-only.** Re-running is a new run, so history is auditable.
- Retention/GC is explicit and bounded — but the **bar-cache eviction lesson applies**: no
  size-capped FIFO that silently destroys history. GC must be opt-in, reported, and refuse
  to delete a dataset that a retained run depends on.

### 5.8 L8 — Portfolio, money management, analysis (see §10)

### 5.9 L9 — Workflow DAG & jobs

- A workflow is a **typed DAG** of stages (`generate → simulate → filter → robustness →
  retest → portfolio-fit → store → report`), authored in the GUI and serialized.
- Stages are **pure functions of (inputs, config, seed)** with content-addressed artifacts,
  so re-running a workflow **resumes from cache** and only recomputes what changed.
- **Bounded concurrency and backpressure** (§12.1). Long runs are cancellable and
  crash-resumable from the last completed stage.
- Runs off the render thread, always ([ADR-134](134-render-independent-background-pump.md)).

### 5.10 L10 — Extensions

- Trait-based extension points: `Indicator`, `Metric`, `Objective`, `Filter`,
  `CostModel`, `Report`, `SearchOperator`.
- Two supply routes: compiled-in Rust, and **transpiled sources via `typhoon-transpiler`**
  (which [ADR-111](111-broker-scope-reduction-kraken-alpaca-only.md) explicitly retained as
  a language tool). No embedded general-purpose scripting runtime in the first pass —
  sandboxing and determinism costs are not yet justified.

### 5.11 L11 — Native GUI

Native egui, consistent with existing conventions:

- **Strategy Builder** — visual IR editor with typed sockets + canonical text view.
- **NNFX Builder** — guided role-based form and operation-profile selector backed by the
  same IR, with per-rule explanations and a one-click transition into the general builder.
- **Run** — dataset picker, cost model picker, progress, cancel.
- **Visual / Hybrid Replay** — play, pause, step, inspect indicator/signal state, and inject
  recorded manual order decisions without leaving the chart.
- **Results** — report with metric definitions on hover, equity + underwater curves, trade
  list, **annotated chart overlay** of entries/exits, MAE/MFE scatter.
- **Optimizer** — parameter-field views: 2D heatmap (exists), 3D surface, parallel
  coordinates for >2 parameters, Pareto front.
- **Databank browser** — filter/sort/compare, saved queries, bounded paging.
- **Portfolio** — correlation matrix, combined equity, capital usage.
- **Workflow editor** — DAG canvas, run history, resume.

---

## 6. Execution realism requirements (normative)

These are **MUST** requirements for the completed L3 simulator. Each ships with golden tests in
the milestone assigned by §13: M1 gates core order/fill, per-trade cost, latency and OHLC
correctness; M2 adds the explicitly deferred richer-execution semantics. Where a policy is
configurable, the **default must be the conservative one**.

### 6.1 OHLC ambiguity policy
Within a bar, the true path is unknown. The simulator must adopt an explicit, recorded
policy and never silently pick the favourable one:
- Default **pessimistic/adverse**: when both a stop and a target are reachable inside one
  bar, assume the **stop** fills first.
- Alternatives selectable and recorded in the run manifest: optimistic, OHLC-path heuristic
  (O→H→L→C for up bars, O→L→H→C for down bars), or intrabar data if available (§6.9).
- Gap handling: an order whose trigger is gapped through fills at the **open**, not at the
  trigger price.
- The chosen policy is a **first-class run parameter** and appears in every report.

### 6.2 Bid/ask & spread
- Simulation prices are bid/ask, not a single mid/close. Buys lift the ask; sells hit the
  bid.
- Spread sources, in preference order: recorded quotes → per-instrument modelled spread
  (constant, session-dependent, or volatility-scaled) → a declared fallback.
- Where only bar data exists, the spread model is **explicit and stamped on the run**;
  "no spread" is a valid but loudly-labelled setting, never the default.

### 6.3 Commissions, fees, funding, borrow
- Per-instrument, per-venue fee schedules for **Kraken and Alpaca** shapes: maker/taker
  tiers, per-share vs percentage-of-notional, minimums, regulatory fees.
- **Overnight funding / financing** for margin positions; **borrow cost** for shorts;
  **crypto funding-rate** semantics where applicable.
- Currency conversion costs where the instrument currency ≠ account currency.
- Fees are applied **at the fill**, not netted at the end, so they compound into the
  drawdown path correctly.

### 6.4 Slippage & latency
- **Decision→submit latency** and **submit→exchange latency** as configurable
  distributions (fixed, uniform, or drawn from a seeded RNG).
- Slippage models: fixed ticks, spread-fraction, volatility-scaled, and size-vs-volume
  impact (§6.6).
- **Every random draw comes from the run's seeded RNG stream** (§6.10) — never
  `thread_rng`.

### 6.5 Order types
Minimum set: `Market`, `Limit`, `Stop`, `StopLimit`, `MarketOnClose`, plus `TimeInForce`
(`IOC`, `FOK`, `Day`, `GTC`, `GTD`), `ReduceOnly`, and bracket/OCO groups. Modification and
cancellation with realistic latency. Rejections (insufficient margin, bad tick size, closed
session) must be **modelled and reported**, not silently dropped.

### 6.6 Partial fills & liquidity
- Fill size capped by a **participation limit** (a configured fraction of bar or window
  volume); the remainder rests, re-attempts, or expires per TIF.
- Optional depth-aware fills where L2 is available ([ADR-129](129-l1-l2-l3-market-data-support.md)).
- **Scaling out** (partial exits, multi-leg targets) is a first-class order concept, not a
  hack around all-or-nothing `Close`.

### 6.7 Sessions, calendars, time zones
- Per-instrument trading calendars: regular/extended sessions, holidays, half-days, and the
  24×5 xStocks vs US-equity distinction ([ADR-110](110-market-session-status-xstocks-24-5-and-us-equities.md)).
- Orders outside a tradable session **queue or reject** per configuration; they never fill.
- All internal time is **UTC**; display and session-relative rules (e.g. "no entries in the
  first 15 minutes") resolve through the instrument's exchange time zone with correct DST.
- A venue's *published* closures, early closes and open overrides are a **source artifact**,
  not a rule: they are stated in exchange-local civil dates, sealed content-addressed with
  every source record and its raw bytes, and outrank the weekday/holiday rule underneath.
- **Authority is a property of the source system, not a caller's claim.** The built-in
  NYSE/xStocks rules are `DerivedRule` and the keyless Yahoo chart endpoint is
  `UnverifiedPublic`; neither may back a run that requires authoritative reference data, and a
  cache inherits exactly the authority of what it cached. An unreachable source is an error,
  never a silent fall back to the rule.

### 6.8 Corporate actions
- Splits, reverse splits, dividends, symbol changes, and delistings applied **as events at
  their effective time**, adjusting open positions and cash — not by silently rewriting
  price history under a live position.
- The dataset's adjustment policy (§5.1) and the simulator's corporate-action handling must
  be **mutually consistent**; the engine must refuse a combination that would double-count
  (e.g. adjusted prices *and* dividend cash events).
- The known merge-layer hazards ([ADR-122](122-curated-stock-split-fallback-for-equity-merge.md),
  [ADR-124](124-depth-era-promotion-must-not-redefine-price-scale.md)) surface as **dataset
  QA warnings** attached to any run built on affected symbols.
- A schedule ingested from a source is a **content-addressed artifact** carrying its source
  system, authority class, completeness claim, covered range, as-of instant and every source
  record with the SHA-256 of its raw bytes. Ratios and dividend amounts cross the boundary as
  exact decimal strings in one canonical spelling, so `2` and `2.0` cannot both mean one split.
  Retrieval time is audit metadata and is deliberately **excluded** from identity: re-fetching
  identical as-of bytes tomorrow is the same artifact and must not change a run id.
- An execution config **binds the artifact ids it was materialized from** (schema v4), so
  changing the reference data changes the config id. A bound calendar id must be the one an
  instrument's calendar was actually sealed from, and vice versa — a binding with nothing
  behind it is a false provenance claim and is refused.

### 6.9 Tick / intrabar modes
Simulation fidelity is a declared, recorded level:
1. **Bar-close** — decisions at close only (today's behaviour, minus its defects).
2. **Bar-OHLC** — intrabar trigger resolution under the §6.1 policy.
3. **Sub-bar** — finer-timeframe bars as the intrabar path (e.g. resolve a 1h bar with 1m
   bars).
4. **Tick** — true tick replay. **Blocked** on tick history, which TyphooN does not store
   today (§11.3); the interface is designed for it, the mode ships when data does.

The level is stamped on the run and shown in every report. Results from different levels are
never compared without the label.

### 6.10 Determinism & seeds
- Every run carries a **root seed**; per-component streams are derived deterministically
  (slippage, latency, Monte Carlo, generator, tie-breaks).
- **Bit-identical repeatability**: same (IR, dataset, config, seed, engine version, and any
  recorded intervention log) ⇒ identical trade list and identical metrics. This is a CI test,
  not an aspiration.
- Parallelism must not affect results: no reliance on completion order, no unordered
  floating-point reductions in the accounting path.
- Wall-clock time and system RNG are **forbidden** in the simulation path.

### 6.11 Multi-symbol synchronization
- One global event clock. Cross-symbol events at the same timestamp resolve by a documented
  deterministic rule.
- **A strategy may only observe data whose timestamp ≤ current event time**, per symbol —
  which correctly models the fact that a slow-updating symbol is genuinely stale, rather
  than back-filling it from the future.
- Multi-timeframe: a higher-timeframe bar becomes visible **only after it closes**. The
  partially-formed current bar is available only via an explicit "forming bar" node with
  its own semantics ([ADR-119](119-live-forming-bar-overlay-source-policy.md)).
- Missing bars/halts are represented as **absence**, never as a carried-forward synthetic
  bar — the Alpaca carry-bar defect is a live example of how carried bars poison analysis.

### 6.12 No look-ahead
- The IR (§5.2) makes future access unrepresentable.
- The simulator additionally enforces it at runtime in debug builds: data access beyond the
  current event time **panics**.
- A standing **look-ahead canary suite** in CI: strategies engineered to cheat (peek at the
  next bar, use the full-series max, use a future-dated indicator) must fail to compile or
  trip the guard.

### 6.13 Decision timing and hybrid replay
- Bar strategies declare a decision point: closed-bar event, next-bar open, or a configured
  pre-close offset. A pre-close rule consumes only the forming-bar state actually available at
  that timestamp and is labelled as such; it may not read the final OHLC values.
- Open-price, sub-bar, and tick modes use the same strategy IR and execution model. Changing
  fidelity must not silently change indicator warm-up, session, or order semantics.
- Manual/hybrid actions are explicit `UserDecision` events (§5.3) and produce a forked child
  run linked to its automated parent. The UI shows which actions came from the strategy and
  which came from the operator.
- A hybrid run may be compared and reported, but it is not eligible for automated promotion
  unless a deterministic rule reproduces the interventions.

---

## 7. Robustness catalog

Nothing here is meaningful until §6 is correct — robustness statistics computed on a
zero-cost, same-bar-fill simulator measure nothing. Robustness work is therefore gated
behind M1/M2.

### 7.1 Out-of-sample schemes
- Multiple, configurable OOS regions — not one fixed 70/30 (§1.2 item 7).
- Supported schemes: leading OOS, trailing OOS, **interleaved/striped** OOS, and multiple
  disjoint OOS windows per run.
- Purging/embargo around split boundaries so that indicator warm-up and trade holding
  periods cannot leak across the seam.
- Every scheme records exactly which bars were IS and which OOS.

### 7.2 Walk-forward
- **Rolling and anchored** walk-forward with configurable window/step, both in bars and in
  calendar time.
- **Walk-forward optimization** — re-optimize each IS window, apply to the following OOS
  window, concatenate the OOS record. That concatenated OOS equity is the headline result,
  not the best IS result.
- **Walk-forward matrix** — sweep (IS length × OOS length) and report the *field* of OOS
  outcomes. A strategy that only survives one specific window pairing is fragile.
- Report per-window and aggregate efficiency, plus **degradation** (OOS vs IS) as a
  distribution, not the single ratio used today.

### 7.3 Monte Carlo families
Each variant has a defined perturbation, a seeded stream, and a confidence-interval output:
- **Trade-order shuffle** — resample the realized trade sequence; tests sequence dependence
  and drawdown luck.
- **Trade skipping / random subset** — drop a fraction of trades; tests reliance on a few
  outliers.
- **Trade resampling with replacement (bootstrap)** — distributional CIs on the headline
  metrics.
- **Parameter perturbation** — jitter parameters within their neighbourhood; a strategy
  that dies under ±5 % is a curve-fit.
- **Data perturbation** — inject noise into prices, randomize bar start offsets, resample
  historical segments (block bootstrap) while preserving autocorrelation structure.
- **Execution perturbation** — randomize slippage, latency, spread, and fill probability
  within the cost model's stated uncertainty.
- **Starting-point / start-date variation** — shift the simulation start; a strategy that
  needs one particular start date is fragile.
- **Randomized order-of-symbol / capital-allocation** for portfolio runs (§10).

Output is always a **distribution with percentiles** (e.g. 5th-percentile CAGR,
95th-percentile max drawdown), never a single number.

### 7.4 Parameter-field analysis
- **Plateau/neighbourhood stability** — the metric surface around the chosen point;
  isolated spikes are rejected.
- **System Parameter Permutation (SPP)** — evaluate across the whole parameter field and
  use the *field's* distribution (e.g. its median) as the performance estimate, instead of
  the single optimized point. Explicitly designed to remove optimization bias.
- **Optimization Profile** — the shape and dispersion of results across the field,
  summarized as a stability score with a documented formula.
- **Views:** 2D heatmap (exists), 3D surface, parallel coordinates, Pareto front.

### 7.5 Cross-checks
Re-run the same strategy id (via the retester, §5.5) against:
- **Other symbols/markets** in the same asset class, and across classes.
- **Adjacent timeframes** (does a 1h edge exist at 30m and 2h?).
- **Alternative data sources** for the same symbol — TyphooN is unusually well-placed here
  ([ADR-113](113-cross-source-equity-bar-merge-data-integrity.md)): a strategy whose edge
  disappears when the Kraken-sourced series is swapped for the Alpaca- or Yahoo-sourced one
  was fitting data artifacts, not market structure.
- **Cost-model sensitivity** — 1×, 2×, 3× the assumed spread/commission.

### 7.6 Degradation gates & problem recognition
Automatic red flags, each an explicit rule with a stated threshold:
- Trade count below a minimum for statistical meaning.
- PnL concentrated in a handful of trades (top-N trade contribution share).
- Edge concentrated in one calendar period, one symbol, or one side (long/short).
- Excessive time in market / near-permanent exposure disguised as an edge.
- Systematic reliance on the very first or last bars of the dataset.
- Absurd metrics (Sharpe far outside plausible range, PF at the sentinel, DD ≈ 0).
- Sensitivity cliff: performance collapses under ±1 parameter step or 2× costs.
- OOS degradation beyond a configured fraction of IS.

Gates produce **verdicts with evidence**, stored in the databank next to the run.

### 7.7 Multiple-testing controls
Testing 100,000 strategies guarantees impressive-looking survivors by chance. Therefore:
- **Every run counts.** The databank records the total number of evaluations in a search,
  so the selection universe is known.
- Report **multiple-testing-adjusted** significance (e.g. deflated performance measures,
  false-discovery-rate control across the candidate set) alongside raw metrics.
- Prefer **SPP/field estimates** (§7.4) over best-point estimates as the headline number.
- The UI must never present "best of N" without displaying N.

### 7.8 Holdout & quarantine discipline
- A **final holdout** segment is defined per dataset and is **inaccessible** to search,
  optimization, and robustness stages — enforced by the dataset API, not by convention.
- Touching the holdout is an explicit, logged, one-way action that **burns** it: the run is
  marked as holdout-consumed, and further tuning against that holdout is refused.
- **Quarantine** — newly promoted strategies run in paper/shadow mode (§10.5) for a
  configured period before any live consideration.

---

## 8. Strategy generation

### 8.1 Typed block grammar, templates, constraints
- Generation samples from the **typed IR grammar** (§5.2), so every generated candidate is
  type-correct and look-ahead-free by construction.
- **Templates** — user-authored partial strategies with holes ("entry: any trend filter +
  any oscillator trigger; exit: ATR stop + fixed R target"), which the generator fills.
  Random placeholders are exactly this: typed holes with a sampling distribution.
- **Constraints** the generator must honour: max node count / depth, allowed indicator set,
  required risk exit (every strategy must define a stop), max simultaneous positions,
  instrument-appropriate sizing, no duplicate sub-expressions, session restrictions.
- **Fuzzy-logic blocks** (graded membership conditions rather than boolean thresholds) are a
  *deferred* grammar extension — recorded as a real vendor-advertised capability, scheduled
  after the core grammar is stable.

### 8.2 Search algorithms — named, conventional, and honest
The generator uses standard, well-understood optimization methods. **No claim is made in
this repository that TyphooN performs "AI" or "machine-learning" strategy discovery beyond
what these named algorithms actually do.**
- **Genetic programming / evolutionary search** over IR trees: tournament selection,
  subtree crossover, point/subtree/parameter mutation, elitism, island populations with
  migration for diversity.
- **Random search** over the grammar — the honest baseline every fancier method must beat.
- **Grid search** for small, fully-enumerable parameter spaces.
- **Bayesian optimization** (TPE/GP) for expensive, low-dimensional continuous parameters.
- **Local search** (coordinate descent, pattern search) for refinement around a plateau.
- **Multi-objective** (NSGA-II-style non-dominated sorting) when objectives genuinely
  conflict.

Machine-learning *components* (e.g. a surrogate model that pre-screens candidates before
full simulation) may be added later, but only with a documented method and a measured
benefit — never as a marketing label.

### 8.3 Novelty & deduplication
- **Structural dedup** via the canonical strategy id (§5.2): syntactically different but
  semantically identical candidates collapse to one.
- **Behavioural dedup**: cluster by trade-level and equity-curve correlation. Two different
  strategies that take the same trades are one strategy.
- **Novelty pressure** in the evolutionary loop: reward candidates that are behaviourally
  distinct from the current archive, to counter premature convergence on one motif.
- The databank tracks **how many times** a strategy id was rediscovered — useful signal,
  and it prevents re-simulating known candidates.

### 8.4 Strategy improver
Take an existing strategy and search its *local* structural neighbourhood — add/remove one
filter, retune one parameter block, swap one indicator of the same type — under the same
robustness gates. Improvements must be validated on data not used for the improvement, or
they are just deeper curve-fitting.

### 8.5 Complexity penalties
- Fitness includes an explicit **parsimony penalty** on node count/depth and on the number
  of free parameters.
- A configurable **minimum-trades** gate and a **degrees-of-freedom vs sample-size** check:
  candidates with too many parameters relative to trades are rejected before robustness
  stages spend time on them.

### 8.6 Deterministic resumability
- A generation run is `(seed, grammar version, constraints, dataset id, config)` — fully
  reproducible.
- Checkpoints after every generation/batch: population, archive, RNG stream state, evaluation
  counter. A killed or crashed run resumes at the checkpoint and produces the same result as
  an uninterrupted run.
- Evaluation results are content-addressed and cached, so a resumed or repeated run never
  re-simulates an already-evaluated (strategy id × dataset × config).

---

## 9. Metrics & reporting

### 9.1 Metric contract
Every metric in the registry has: a stable id; a **written formula**; units; a periodicity
and annualization rule; defined behaviour for the empty/degenerate case (**no silent
sentinels like today's `999.0` profit factor** — degenerate is `None`/undefined and renders
as such); and a golden test with hand-verified expected values. The registry is versioned;
`metrics_version` is stamped on every run, and cross-version comparisons are labelled.

### 9.2 Required metric families

**Return & profit:** net profit, gross profit/loss, CAGR, total return, return on max DD,
average trade (absolute, %, and R-multiple), expectancy, profit factor (with a defined
undefined-case), payoff ratio.

**Risk & drawdown:** max drawdown (absolute, %, **and duration**), average drawdown,
underwater curve, ulcer index, time-to-recovery, longest **stagnation** period (time at or
below a prior equity peak), max adverse portfolio excursion.

**Ratios:** Sharpe (**periodicity and risk-free rate explicit**), Sortino, Calmar, MAR,
Sterling, K-ratio, R-squared of the equity curve against its linear fit.

**Trade-level:** MAE (maximum adverse excursion) and MFE (maximum favourable excursion) per
trade, with scatter plots and efficiency ratios (how much of the favourable excursion was
captured) — the direct evidence for stop/target placement.

**Exposure & activity:** time in market %, average holding period, trades per period,
long/short split, max concurrent positions, capital utilization, turnover.

**Distribution & tails:** trade PnL distribution with skew/kurtosis, win/loss streak
distributions, VaR/CVaR of periodic returns, worst-N-day loss, tail ratio.

**Stability:** rolling-window metric series, per-year/per-quarter/per-month breakdown,
IS-vs-OOS deltas, equity-curve linearity, performance concentration (share of PnL from the
top decile of trades).

**Calendar:** daily/weekly/monthly/annual equity and return series — a real
calendar-resampled series from mark-to-market equity, which today's per-trade-close curve
cannot produce.

**Uncertainty (mandatory, not optional):** confidence intervals on the headline metrics
from the Monte Carlo families (§7.3), the evaluation count N behind any "best" claim
(§7.7), and the standard error of the mean trade. **A point estimate presented without its
uncertainty is considered a reporting bug.**

**Diagnostics:** cost breakdown (commission/spread/slippage/funding as a share of gross
PnL), rejected/expired/unfilled order counts, session-blocked signal counts, warm-up bars
consumed, and the count of bars where data was absent.

### 9.3 Reports
Deterministic, versioned, exportable (JSON + human-readable), with the full run manifest
attached: IR hash, dataset id, cost model, fidelity level, seed, engine version, metrics
version. **A report without a reproducible manifest is not a report.**

---

## 10. Portfolio parity

### 10.1 Correlated strategies
- Correlation of returns, of trade timing, and of drawdown periods across strategies.
- Clustering to identify redundant strategies (same edge in different clothes).
- Portfolio metrics computed from the **combined simulated equity**, not by summing
  individually-simulated PnL — because shared capital and margin change the result.

### 10.2 Fit-to-portfolio
Selecting the individually-best strategies produces a correlated, fragile portfolio. The
objective must be **marginal contribution to the portfolio**: does adding this strategy
improve the portfolio's risk-adjusted return, reduce its drawdown, or cover an uncovered
regime/market/session? Selection is a portfolio-level search (greedy forward selection plus
a diversity constraint, with the option of a full multi-objective search), not a top-N sort.

### 10.3 Capital, margin, and money management
- Shared capital pool with per-strategy allocation, priority, and caps.
- Margin/buying-power modelling with rejection when insufficient (not silent over-leverage).
- **Concurrent-fill contention:** two strategies competing for the same capital at the same
  instant resolve by a documented deterministic rule, and the loser is genuinely rejected.
- Money-management schemes as pluggable sizing models: fixed lot, fixed fractional,
  fixed-ratio, volatility-targeted/ATR-based, Kelly-fraction (with an explicit cap and a
  loud warning). **Martingale-style escalation is prohibited** for anything that can reach
  live trading ([ADR-114](114-deprecate-martingale-live-trading.md)); if simulated at all it
  is labelled research-only and blocked from promotion.
- **NNFX two-leg template:** independently configurable first/second leg risk share,
  ATR-derived or fixed SL/TP, break-even trigger, trailing-stop activation/distance/step, and
  optional single-order virtual targets. These are ordinary typed order-management nodes, so
  reports and optimizers can inspect them rather than opaque special cases.
- **Overexposure policy:** configurable asset/currency exposure groups can block an order or
  reduce its risk when it repeats the same underlying currency/asset or directional
  exposure. The policy declares which account(s), manual/strategy orders, break-even/zero-risk
  positions, and excluded symbols it considers. It uses canonical instrument metadata, with
  explicit user-defined groups only when the venue model cannot express the relationship.

### 10.4 Portfolio stress, Monte Carlo, and what-if
- All §7.3 Monte Carlo families applied at the **portfolio** level, plus allocation-order
  and strategy-subset randomization.
- **What-if:** re-evaluate a stored portfolio under changed assumptions — different starting
  capital, sizing model, cost model, or with strategies added/removed/disabled — without
  re-running the underlying simulations where trade-level results can be reused.
- **Equity-control simulation:** rules that scale or suspend a strategy based on its own
  equity curve (e.g. stop after N losses, resume above a moving average of equity),
  evaluated honestly — including the case where the control rule *hurts*.

### 10.5 Lifecycle, paper shadow, and re-validation
- Strategy states: `candidate → validated → quarantined (paper/shadow) → active → retired`,
  with the transition criteria recorded.
- **Paper shadow validation:** a promoted strategy runs against live data through the normal
  broker path in paper mode ([ADR-126](126-primary-assist-broker-selection.md),
  [ADR-130](130-multi-account-broker-support.md)), and its **live-vs-simulated divergence**
  (fills, slippage, timing) is measured and fed back into the cost model. This closes the
  loop that the removed "final MT5 validation" step in ADR-038 used to occupy — with the
  crucial difference that it validates against **the venue we actually trade**.
- **Scheduled re-validation:** periodic retest on newly arrived data; a degradation gate
  breach demotes the strategy and raises an alert.
- The same immutable strategy id/config drives backtest, hybrid replay, paper shadow, signal
  notification, assistant/manual execution, and (only after explicit user enablement) live
  automation. No copy-and-paste configuration fork is allowed between stages.
- Promotion never means automatic deployment: live eligibility, account selection, sizing,
  and automatic-vs-assistant mode remain explicit graphical controls with audit records.

---

## 11. Data QA & reproducibility

### 11.1 Dataset build & verification
- Materializing a dataset runs a **QA pass** and stores the report with the dataset:
  gaps vs the instrument calendar, duplicate/out-of-order timestamps, zero-volume and
  carry-forward bars, spikes/outliers vs a robust volatility band, OHLC invariant violations
  (`low ≤ min(open, close) ≤ max(open, close) ≤ high`), impossible/negative prices, and
  suspicious level shifts (split-like discontinuities).
- The manifest hash covers both the data and the QA report — a dataset that fails a
  configured QA threshold **cannot be silently used**; the run must acknowledge it, and the
  acknowledgement is recorded in the report.
- Timeframe transforms (resampling to higher TFs) are deterministic, documented, and
  labelled as derived, carrying the source TF's provenance.

### 11.2 Inspection tools
A tabular data browser (bar-level, with QA flags highlighted) alongside the chart, so a
suspicious backtest result can be traced to specific bars — the missing half of "chart and
table inspection".

### 11.3 Tick data — **Blocked**
Real-tick backtesting is a genuine capability gap with an honest blocker: **TyphooN does not
store a versioned historical tick corpus today**. The depth, retention, and entitlement of
candidate Kraken/Alpaca historical-trade sources have not yet been established as sufficient
for reproducible tick testing. Live L1/L2 exists
([ADR-129](129-l1-l2-l3-market-data-support.md)) and could be *recorded forward* without a
new paid dependency. The simulator's fidelity ladder (§6.9) is designed so tick mode drops
in when a suitable corpus exists; until then the highest honest fidelity is sub-bar (§6.9
level 3).

### 11.4 Custom import — **Deferred**
Third-party file import (CSV/HDF) is straightforward but not on the critical path; it lands
after the dataset abstraction is stable, so imports arrive as first-class provenance-carrying
datasets rather than as a side door around QA.

### 11.5 Indicator and calendar diagnostics
- **Repainting test:** evaluate each indicator incrementally, snapshot outputs that were
  visible at each event, and fail/report when a later event mutates a previously closed-bar
  value outside an explicitly declared revision window. Report the exact node, output, bar,
  old value, and new value; do not reduce this to a single warning flag.
- **Weekend-candle test:** compare timestamps against the instrument's declared calendar and
  identify unexpected weekend/session bars. Crypto weekend bars are normally valid; an FX,
  equity, or xStock bar is judged against its own venue/session policy rather than a global
  weekend rule. M0 stores the dataset-side calendar id/policy used by this QA check (initially
  reusing the existing ADR-110 session/calendar rules); M2 makes the simulator's execution
  calendar consume that same versioned policy.
- Both diagnostics are dataset/indicator QA artifacts stored with the run and visible in the
  chart/table inspector. A user may acknowledge a warning, but the acknowledgement is part of
  the manifest.

---

## 12. Performance, resources, and the role of the GPU

### 12.1 Bounded memory & backpressure
- Simulation streams bars; it does not require the full history in RAM. Large sweeps run in
  **bounded batches** with a hard cap on in-flight work.
- Result streams are written to the databank incrementally with **backpressure**; a
  generator that outproduces the evaluator blocks rather than growing an unbounded queue.
- Memory limits are configured and enforced; exceeding them **degrades gracefully**
  (smaller batches) rather than OOM-ing the terminal.

### 12.2 O(1) UI discipline
- The databank browser and every strategy window query through **indexed** paths with
  bounded result windows — the research-snapshot lesson applies directly: a per-strategy
  detail view is an indexed on-demand query, never a scan.
- No simulation, no database walk, and no metric aggregation on the render thread
  ([ADR-098](098-per-frame-o-1-discipline-in-chart-and-sync-paths.md),
  [ADR-134](134-render-independent-background-pump.md)). Long work runs on background
  workers and reports via channels.

### 12.3 GPU is an accelerator, never the definition
- The **CPU reference interpreter is definitional**. A GPU path may only be used for a
  workload after an **equivalence test** proves it matches the CPU reference within a
  declared tolerance on a golden corpus.
- The current fixed SMA/NNFX shaders are **not** a general evaluator; the general path is
  WGSL emitted from the IR (§5.2), and it must pass the same equivalence tests.
- `f32` GPU vs `f64` CPU divergence is a **known and now unmitigated** risk: ADR-038's
  mitigation was "final MT5 validation," and MT5 is gone ([ADR-111](111-broker-scope-reduction-kraken-alpaca-only.md)).
  The replacement mitigation is internal: mandatory CPU re-verification of any GPU-selected
  candidate before it enters the databank as validated.
- **No performance claims** are made in this ADR. Speedups are to be measured on this
  hardware, on this workload, and recorded with the measurement method — see §15 on
  ADR-038's unsourced numbers.

---

## 13. Staged delivery plan

Milestones are **gated by acceptance criteria and prerequisites, not by dates**. A milestone
is not "done" until its gate passes; later milestones do not begin early on the assumption
that an earlier gate will pass.

### 13.1 Implementation ledger (2026-07-28)

This ledger distinguishes landed foundations from milestone completion. The acceptance gates
below remain authoritative; a checked foundation does **not** imply that its milestone is done.

**Landed on `master`:**

- `strategy_dataset.rs`: immutable content-addressed dataset manifests, canonical finite-float
  identity, provenance, deterministic QA, and tamper verification. The QA pass now covers the
  full M0 defect corpus — gaps, robust-band price spikes, duplicate/out-of-order timestamps,
  carry-forward bars, OHLC violations, split-like level shifts, and unexpected
  weekend/holiday/out-of-session bars — under a versioned four-variant calendar policy
  (`Continuous24x7`, `WeekdaysOnly`, `UsEquityRegular`, `XStock24x5`, reusing the ADR-110
  holiday rules) and a versioned, identity-bearing `DatasetQaPolicy`. Identity is split in
  two: `dataset_id` addresses the exact persisted bar bytes (including signed-zero bits), and
  `manifest_id` seals it together with the
  calendar policy, the QA policy, and the QA report hash (§11.1). Findings are capped by the
  policy and report their own truncation.
- `strategy_dataset_store.rs`: the filesystem storage boundary — a content-addressed record
  (`manifest.json`, `qa.json`, `bars.bin`) published atomically by `fsync` + directory rename,
  with a purpose-built payload format that preserves exact `f64` bit patterns and verbatim
  timestamps, carries a trailing offset index for O(1)-ish paged reads, and is digest-verified
  on open. Loading is bounded on every axis (bar count, artifact bytes, page size, listing
  size, timestamp length) and dataset ids are validated before any path is built from them.
  Linux store traversal is descriptor-relative and fail-closed through `openat2` beneath/no-follow
  constraints; publication synchronizes the layout, staging record, shard, and layout metadata in
  crash-durable order, including first-shard creation. Other targets retain the same compiling API
  but `FileDatasetStore::open` returns `UnsupportedPlatform` until an equivalent hardened boundary
  is implemented.
- `strategy_dataset_worker.rs`: dataset construction, QA, storage, and paged reads on a
  dedicated thread behind bounded job/event queues, non-blocking submission with explicit
  backpressure, stage-granular cancellation, error delivery as events, and a bounded
  open-record cache.
- `typhoon-native` Dataset Inspector (`dataset_inspector_model.rs`,
  `floating_windows/dataset_inspector.rs`): a graphical tabular browser over stored datasets
  showing manifest, provenance, calendar/QA policy ids, QA counts, and per-bar QA flags. The
  render path draws only the page the worker delivered, virtualized by `show_rows`; it opens
  no store, walks no database, and aggregates nothing. Active-chart snapshot conversion allocates
  at most one fixed-size chunk per frame; the chunked snapshot transfers without flattening or
  input-sized copying on render and is flattened only by the worker. An O(1) bars generation is
  advanced on production replacements, appends, and in-place OHLCV updates, so same-length,
  same-timestamp chart mutations cancel materialization fail-closed.
- `strategy_ir.rs`: canonical strategy IR, semantic/type/resource validation, content-addressed
  strategy and execution identities, bounded persisted-artifact loading, and stable identity
  vectors. Execution-config schema v3 seals the participation model, per-instrument calendars,
  quote currencies/ticks/financing, corporate-action schedule, conversion table, closed-session
  policy, and declared sub-bar timeframe. Schema v4 additionally seals the reference-data
  artifact ids the settings were materialized from. The bump is a compatible extension rather
  than a reinterpretation: v3 remains loadable and keeps its sealed id byte for byte, because
  bindings are hashed only from v4 onwards, and a v3 artifact carrying bindings is refused so
  the field cannot be smuggled outside the id that covers it. True tick fidelity is deliberately
  unrepresentable while TyphooN retains no versioned tick history.
- `strategy_calendar.rs`, `strategy_instrument.rs`: bounded, content-addressed per-instrument
  execution calendars and canonical instrument specs. UTC remains the internal clock; US-Eastern
  windows resolve under the shared ADR-110 DST/holiday rules, including the documented rule-based
  early closes. At every execution instant a closed venue prevents a fill, while submission either
  queues or produces an explicit rejection according to the sealed policy. A calendar may now
  also carry a bounded, ascending, duplicate-free set of *published* exceptions keyed by
  exchange-local date, sealed into `calendar_id` alongside the artifact id they came from. A
  published verdict outranks every derived rule for that date and reports as
  `ClosedReason::PublishedClosure`, distinct from the rule-derived `Holiday`, so a report never
  presents a guess as an exchange statement. Exceptions and their artifact id are present
  together or not at all.
- `strategy_reference_data.rs` (M2): the §6.7–§6.8 ingestion boundary. It does not fetch; it
  accepts a bounded snapshot of records some other process persisted and seals a verified
  calendar-exception or corporate-action artifact from them. Authority is derived from the
  source system rather than taken on trust, so a keyless feed cannot be labelled official and a
  cache is worth exactly what it cached; incomplete, uncovered, unreachable, malformed,
  unsupported, duplicate, conflicting, out-of-range, offsetless and adjusted-price-double-count
  inputs all fail closed. Identity covers source class, authority, completeness, covered range,
  as-of instant, scope, every source-record id with the SHA-256 of its raw bytes, and the
  canonical executable events; retrieval time is excluded by a named, hashed decision.
  Materialization sorts source records into the same canonical order the sealed events use, so
  the two lists are one order rather than two. The codec bounds bytes before parsing, denies
  unknown fields, and re-serializes to prove the input was the one canonical encoding, so a
  reordered or prettified file cannot share an id. The store is a flat content-addressed
  directory addressed only by 64-hex id — never by caller path — with write-then-rename
  publication.
- `strategy_reference_data_worker.rs` (M2): the background boundary for the above. Every
  snapshot read, digest, verify, materialize, list and bind runs on one named thread behind a
  bounded job queue and a bounded event queue with a per-poll ceiling; submit and poll never
  block a frame callback. Inspection is a dry run that reports the exact refusal a promotion
  would hit, so a blocked snapshot disables promotion *and* says why instead of offering an
  action that will fail. Listings report what they omitted. Selection re-loads both artifacts
  through the store — re-verifying them against their ids — before binding and sealing a config,
  and labels the result with the authority it actually has rather than the fact that it sealed.
- `strategy_financing.rs`: identity-bearing constant-rate assumptions with provenance for long/
  short financing, stock borrow, crypto funding and quote-to-account currency conversion. Accrual
  boundaries are deterministic events and conversion costs are applied per fill. `Unavailable` is
  distinct from `NotApplicable`: exposure requiring an unavailable paid/live input fails closed;
  the engine never converts that absence to a zero rate.
- `strategy_corporate.rs`: canonical effective-time split, cash-dividend, symbol-change and
  delisting schedules. Verified run assembly now joins the config to the bound dataset adjustment
  policy and rejects a split/dividend event already baked into split-adjusted or total-return
  prices, closing the §6.8 double-counting boundary rather than merely exposing a helper check.
- `strategy_interpreter.rs`: bounded scalar lowering from canonical IR into the simulator for
  closed-bar fixed-unit strategies, deterministic built-in indicator state, three-valued
  conditions, and explicit rejection of semantics the simulator cannot yet honor.
- `strategy_simulator.rs`: bounded scalar multi-symbol event ordering, closed-bar decisions with
  next-open market fills, explicit spread/slippage/commission accounting, no-look-ahead market
  views, deterministic event/ledger JSON, and a stable golden-ledger digest. The richer-execution
  kernel adds a parent-bar participation budget shared across all orders and sub-bars, partial-fill
  remainders with IOC/FOK/OCO handling, fill-time session gating, effective-time corporate events,
  time-accrued financing/borrow/funding, per-fill currency conversion, per-instrument price ticks,
  and fail-closed level-3 sub-bar paths. The focused sub-bar golden proves that an earlier finer-bar
  target beats a later stop even when the enclosing parent bar's pessimistic ambiguity policy would
  choose the stop.
- `strategy_run.rs`: named dataset-input bindings and cross-artifact assembly that verifies the
  strategy, execution config, run manifest, dataset manifests, actual bar content, optional
  intervention log, and every manifest-bound repaint QA artifact before a run can be treated as
  resolved. Repaint artifacts are resolved in canonical indicator-id order and missing,
  duplicate, unexpected, tampered, foreign-dataset, or acknowledgement-inconsistent evidence is
  rejected fail-closed.
- `strategy_metrics.rs` (M2, partial): a versioned 46-metric registry where every entry carries
  an id, written formula, units, periodicity, annualization rule and degenerate-case contract
  (§9.1). Values are a typed `Defined`/`Undefined { reason }` pair — there is no `999.0`-style
  sentinel and no NaN or infinity, because a non-finite result is converted to
  `Undefined { ArithmeticOverflow }` at construction. Covers return/profit, risk/drawdown,
  ratios (Sharpe, Sortino, Calmar, Sterling, equity-curve R², K-ratio), per-trade MAE/MFE and
  capture efficiency, exposure/activity, distribution and tails (skew, excess kurtosis, streaks,
  VaR/CVaR, tail ratio), stability concentration, real calendar resampling to daily/weekly ISO/
  monthly/annual series, and a ledger cost/rejection diagnostic block. Uncertainty is typed
  rather than omitted: the mean-trade standard error is reported and Monte-Carlo confidence
  intervals are an explicit `UnavailableUntilM4` value instead of a silent absence. Compact
  hand-derived ledgers now assert an exact numeric value or exact typed undefined reason for every
  registry id; a set-equality test prevents a registry addition from escaping that golden corpus.
- `strategy_protective.rs` + `strategy_interpreter.rs` (M2, partial): canonical IR
  `TradeManagement` now lowers into the NNFX two-leg order lifecycle (§10.3), resolving fixed,
  percent-of-entry, and ATR-multiple rules from deterministic interpreter state. The lifecycle
  is a pure state machine over the M1 simulator — N legs each with an independent stop and target in
  their own OCO group, a position-level break-even move, per-leg trailing that only ever
  tightens, and a bar-budget time stop. It is expressible because orders execute within a bar
  in submission order and `reduce_only` is checked at execution, so a bracket submitted with
  its entry protects the entry bar itself. Protective levels are anchored to the strategy's
  reference price rather than the achieved fill, so spread, slippage or a gap shows up as
  execution cost instead of silently resizing the authored risk. To support it,
  `DecisionContext` gained a `PositionView` (units, average entry, realized PnL, entry time,
  and a committed-bars-only favourable extreme) — the feedback whose absence previously forced
  the interpreter to reject break-even, trailing and time stops outright. Canonical authored exits
  retire still-live brackets on the same decision, and time-stop exits take precedence over a
  competing authored exit. This implementation is pending the final M2 verification command set;
  it does not complete M2.
- `strategy_intervention.rs` (M2): the operator intervention log and deterministic hybrid
  replay (§6.13). Every operator action is anchored to the *decision ordinal* it interrupted
  rather than to a wall clock — the decision sequence is already the run's deterministic spine,
  so replay needs no clock and cannot drift on re-resolution. Operators act through the same
  submit/cancel/modify interface as an automated strategy, which is what makes their actions
  reproduce with the same fidelity. The log is content-addressed over its interventions
  including the operator's stated reason, so reordering two same-decision actions, moving one
  to a different decision, or rewriting a note all change its id. Loading is byte-bounded,
  version-checked and re-verified. Verified run assembly now requires the exact log named by
  `RunBinding::intervention_log_id`, rejects a missing or mismatched log, and rejects a log supplied
  to an automated manifest. This assembly wiring is pending final verification. A UI to produce a
  log from a live session remains open.
- `strategy_repaint.rs` (M2): the §11.5 repainting diagnostic. It evaluates an indicator over
  every prefix of the bar series and reports each already-published closed-bar value that a
  later event moved, naming the output, the bar, the event, and both values — never a single
  "repaints" flag. Legitimate provisional output is expressible but must be *declared*: an
  undeclared revision window is zero, and a window narrower than the indicator's true
  look-ahead still fails. A value that disappears counts as a repaint, findings are capped
  with the omission reported, the scan is bounded before it runs, and a misshapen indicator is
  rejected rather than compared. The exact report is now sealed into a domain-separated,
  content-addressed, byte-bounded persisted QA artifact with strict nested decoding, structural
  bounds, deterministic finding order, and typed undefined values. Run-manifest v4 binds each
  artifact id plus a `Clean` or noted `WarningAcknowledged` disposition into `run_id`; verified
  assembly proves that disposition agrees with the resolved report.
- `strategy_report.rs` (M2, partial): the sealed report artifact. `report_id` is domain-separated
  over the schema version, verified run id, metrics version, and separate digests of the
  simulator report and its event stream, with the analysis sealed in its exact JSON-wire
  interpretation so a freshly built artifact still verifies after round-trip. Loading is byte-
  bounded before decode, structurally validated against the registry, and fails closed on a
  tampered body, an unsealed id, a foreign run, or a replay whose ledger digest differs. Because
  it inherits the verified manifest's `run_id`, repaint artifact identity and acknowledgement
  changes propagate into report identity.

**M0 gate — passed (2026-07-27):**

Each clause of the M0 gate is now proven by a test rather than asserted:

| Gate clause | Evidence |
| --- | --- |
| A dataset id reproducibly materializes byte-identical bars across restarts | `strategy_dataset_store/tests.rs::a_stored_dataset_recovers_byte_identically_across_a_restart` — stores, drops the store handle, reopens the root, and compares `f64::to_bits` per field plus the re-encoded payload bytes, on a corpus containing `-0.0`, a subnormal, `f64::MAX`, and timestamps of differing byte length |
| QA detects every seeded synthetic defect at 100 % | `strategy_dataset/tests.rs::qa_detects_every_seeded_defect_class` — one seeded case per class (gap, spike, duplicate, out-of-order, carry bar, OHLC violation, split-like shift, weekend, holiday, out-of-session) plus an undamaged control that must stay finding-free |
| Building a dataset never blocks the render thread | `strategy_dataset_worker/tests.rs::dataset_work_runs_off_the_submitting_thread` asserts the executing `ThreadId` differs from the submitter's; `submitting_and_polling_never_block_the_caller`, `the_job_queue_is_bounded_and_reports_backpressure`, and `each_poll_drains_at_most_one_bounded_batch` pin the non-blocking/bounded contract |

**Remaining M0-scope refinements (deliverable polish, not gate blockers):**

- User-driven timeframe transforms (§11.1): resampling a stored dataset to a higher timeframe
  as a *derived* dataset carrying the source timeframe's provenance is not implemented. The
  existing sync/merge derivation is not exposed as a dataset tool.
- Range/symbol selection: datasets are materialized from the active chart's already-loaded
  bars. There is no picker that streams an arbitrary (symbol, timeframe, UTC range) straight
  out of the SQLite cache, so the dataset's range is whatever the chart holds.
- Retention/GC for the artifact store (§5.7) is not implemented; records accumulate until
  removed by hand. The bar-cache eviction lesson applies — any future GC must be opt-in and
  reported, never a silent size-capped FIFO.
- Calendar coverage stays coarse by design: no early-close/half-day handling and no per-symbol
  xStock 24×7 tier, both of which are on ADR-110's own deferred list.

**M1 gate — passed (2026-07-27):**

| Gate clause | Evidence |
| --- | --- |
| Hand-computed golden corpus | `strategy_simulator/tests/golden.rs` pins commission/spread, stop-and-target ambiguity under all policies, stop gaps, never-filled limits, reversal costs, latency and warm-up boundaries |
| Bit-identical determinism | `strategy_simulator/tests/determinism.rs` compares serialized ledgers across repeats, concurrent threads, interleaved runs and seeded-latency streams; `reference_ledger_v1_golden_digest_is_stable` pins the canonical ledger digest |
| Look-ahead canaries | `strategy_simulator/tests/lookahead.rs` proves closed-bar, pre-close/forming-bar, future-index, whole-series and higher-timeframe access cannot reveal uncommitted values |
| Five-strategy zero-cost equivalence | `strategy_simulator/tests/legacy_equivalence.rs::canonical_ir_matches_all_five_legacy_strategies_end_to_end` builds and seals canonical IR, executes `CanonicalIrStrategy`, and compares exact entry/exit prices, realized PnL and final equity against fresh legacy SMA Cross, NNFX, KAMA Cross, Fisher Cross and RSI Mean Reversion runs under the explicit `LegacySameBarClose` bridge |
| Visible cost sensitivity | `strategy_simulator/tests/golden.rs::golden_cost_sensitivity_is_ordered_and_material` pins correctly ordered 0×/1×/2× outcomes |

The compatibility bridge is named and isolated: realistic configurations cannot silently opt into
same-close execution. Venue schedules are identity-bearing operator/vendor artifacts; built-in
tests use explicit assumptions rather than claiming that checkout-time constants are current.
Identity-bearing execution goes through `run_verified_simulation`: seed, decision point and
submission delay are derived from the verified manifest and strategy IR, so no mutable setup can
silently disagree with the published run id. The raw simulator entry point remains available only
as an explicitly non-identity-bearing kernel API for tests and exploratory callers.

**M2 — gate NOT passed; metrics/report and richer-execution foundations landed.** The following
M2 clauses and delivery slices now have test evidence. This matrix is implementation accounting,
not a claim that the complete native M2 workflow exists:

| M2 gate clause | Status | Evidence |
| --- | --- | --- |
| Every metric has a written definition | **Met** | `strategy_metrics.rs::REGISTRY` gives all 46 metrics an id/formula/units/periodicity/annualization/degenerate-case contract; `registry_defines_every_metric_contract_without_duplicate_ids` rejects blank fields and duplicate ids, and `registry_definitions_are_pinned_to_the_metrics_schema_version` digests the whole registry so a definition edit cannot ship without a `METRICS_SCHEMA_VERSION` bump |
| …and a hand-verified golden test | **Met** | The compact corpora in `strategy_metrics.rs::tests` assert a hand-derived numeric value or exact typed `UndefinedReason` for every registry id, including trade/distribution/exposure, tails/drawdown, straight-line regression, exact one-year CAGR, and MAE/MFE ledgers. `hand_verified_metric_corpus_covers_every_registry_id_exactly_once` compares the 46 covered ids with the live registry, so an unpinned registry addition fails the gate |
| No metric returns a magic sentinel | **Met** | `MetricValue` is a typed `Defined`/`Undefined { reason }` pair and `MetricValue::defined` converts any non-finite result to `Undefined { ArithmeticOverflow }`; `a_flat_curve_reports_typed_undefined_rather_than_a_fabricated_ratio` asserts the specific reason for nine degenerate ids and that no `-0.0` reaches the wire; `every_registered_metric_is_computed_rather_than_silently_defaulted` proves the id set is exactly the registry, so a typo cannot masquerade as "not enough data" |
| Calendar equity reconciles exactly with the trade list | **Met** | `calendar_equity_reconciles_exactly_with_the_closed_trade_list` asserts summed daily changes equal summed closed-trade PnL equal `net_profit`; `every_calendar_granularity_reconciles_with_the_overall_equity_change` extends the identity to the weekly/monthly/annual series |
| MAE/MFE verified against hand-computed excursions | **Met** | `long_and_short_mae_mfe_exclude_pre_entry_and_post_exit_bars` pins both directions and the capture ratio, and proves bars outside the holding window are excluded |
| Reports round-trip through JSON without loss | **Met** | `report_artifact_round_trips_detects_tampering_and_rejects_replay_mismatch` round-trips, re-verifies, and rejects both a mutated metric value and a divergent replay ledger; `report_loader_is_byte_bounded_before_decode` and `an_unsealed_report_id_never_passes_verification` pin the fail-closed loader. Report schema v2 adds the exact run-manifest evidence required by comparison views, while `sealed_v1_golden_bytes_and_report_identity_remain_loadable_and_unchanged` pins legacy v1 bytes and report id so previously sealed artifacts remain loadable without identity rewriting |
| Native report/results and chart-overlay slice | **Partial; verified viewer, bounded comparison/distributions, sub-bar runs, and intervention promotion landed** | The Backtest Engine graphically selects one sealed report artifact plus its paired `SimulationReport` JSON and performs bounded reads, decoding, digest verification and presentation indexing on a worker. It provides summary/metric/trade/equity/drawdown sections; bounded worker-precomputed daily close-to-close return, holding-duration, MAE and MFE histograms; run evidence for repaint acknowledgements, intervention logs and sub-bar bindings; identity-preserving report/simulation export; and bounded authoring, loading, sealing and export of intervention logs at actually revealed causal decision indices. A separate stale-safe worker loads two to four complete artifact/simulation pairs under per-file and aggregate byte caps, rejects ambiguous pairing and duplicate identities, validates every pair independently, preserves registry order, and shows numeric deltas only when both typed metric values are defined; a selected verified run can reuse the existing chart overlay. Candidate logs are promoted only through the separate manifest-bound replay path: the user selects sealed Strategy IR/config/manifest/log artifacts and every bound parent dataset, a bounded worker re-loads the exact store records, requires `manifest.intervention_log_id == log.log_id`, assembles with `assemble_verified_run_with_intervention`, executes `run_verified_simulation_with_intervention`, seals and verifies the report, snapshots chart presentation data, rejects stale/cancelled completions, and only then replaces the installed report. Full run/log/report identities and fail-closed errors remain visible. Worker-side export re-verifies intervention bytes; replay hides completed-run summaries/metrics/curves/trade lists, future open-trade outcomes and future chart overlays. StrategyQuant-style Monte Carlo, rolling and parameter-surface analysis belong to M4 rather than this M2 report slice |
| Replaying a recorded hybrid run reproduces its ledger bit-for-bit | **Met at engine and native promotion boundaries** | `strategy_intervention/tests.rs::replaying_a_recorded_hybrid_run_reproduces_its_ledger_bit_for_bit` records a session where an automated strategy and an operator both act under costs, intrabar resolution and seeded latency, then replays the sealed log and compares the serialized ledgers byte for byte. `verified_hybrid_execution_is_identity_bound_exact_and_fail_closed` additionally requires the supplied log id to match the verified run manifest, proves exact repeated output and actual application, and rejects missing, unexpected, mismatched and trailing interventions; malformed loaded actions/content identities fail before assembly/replay. Native real-store worker tests cover successful report preparation/promotion plus missing, unexpected, mismatched, malformed, trailing, stale and cancelled paths while preserving the prior installed report. `a_hybrid_run_replays_identically_from_a_round_tripped_log` proves persistence, and `the_operator_actions_are_what_make_the_two_ledgers_match` is the non-vacuous control |
| A synthetic repainting indicator is identified at the exact mutated bar/output | **Met** | `strategy_repaint/tests.rs::a_synthetic_repaint_is_identified_at_the_exact_bar_and_output` asserts the exact output name and index, mutated bar, responsible prefix, distance, and before/after values; the centred/trailing/disappearing/revision-window controls cover the diagnostic semantics. `stored_qa_artifact_round_trips_with_identity_and_undefined_values` and `stored_qa_artifact_rejects_tampering_unknown_fields_and_oversize_before_decode` seal the evidence and pin bounded fail-closed loading. `verified_run_assembly_binds_acknowledged_repaint_qa_fail_closed` proves exact artifact resolution, dataset binding, clean/warning consistency, and rejection of missing, duplicate, mismatched, or unexpected evidence; `report_identity_inherits_the_manifest_repaint_artifact_and_acknowledgement_binding` proves propagation into report identity |
| Hand-computed two-leg scenario (independent target/stop, break-even, trailing, fee, ledger effect) | **Met** | `strategy_protective/tests.rs::a_hand_computed_two_leg_trade_banks_its_target_moves_to_break_even_then_trails` derives every fill price, fee, realized PnL, cash balance and final equity on paper and asserts them exactly, including leg 0 banking its own target while leg 1 runs, the break-even move at +4.00 and the trailing step at +7.00. `both_legs_stop_out_independently_at_the_same_initial_level`, `a_short_two_leg_trade_mirrors_the_long_lifecycle`, `a_trailing_stop_never_loosens_on_a_pullback`, `a_time_stop_closes_the_remainder_at_market`, `a_legs_stop_and_target_retire_each_other_through_their_oco_group` and `a_strategy_exit_leaves_the_manager_to_cancel_the_resting_bracket` pin the state machine. `strategy_interpreter/tests.rs::canonical_trade_management_compiles_fixed_percent_and_atr_distances` pins resolved fixed, percent, and ATR distances and leg quantities; `canonical_two_leg_management_executes_a_real_partial_target`, `canonical_strategy_exit_retires_its_protective_orders_without_a_later_decision`, and `canonical_time_stop_wins_over_an_exit_signal_on_the_same_decision` cover the canonical-IR execution boundary. These passed in the complete `typhoon-engine` suite recorded for this checkpoint |
| §6.6 partial fills and liquidity cap | **Foundation landed; native results workflow open** | `strategy_simulator/tests/richer_execution.rs::participation_cap_partially_fills_and_preserves_remainder` pins 5/5/2 fills from one 12-unit order under one shared volume budget, exact remaining quantities, partial-fill events and final position. The implementation also handles IOC/FOK and proportionally consumes OCO siblings; optional L2 depth-aware execution remains open |
| §6.7 sessions, calendars and time zones | **Foundation landed; exception ingestion/materialization and native selection landed; source feeds open** | `closed_session_rejects_or_queues_without_an_out_of_session_fill` proves both configured policies and the first valid reopen fill; `strategy_calendar/tests.rs::us_equity_sessions_are_dst_correct_and_half_open` pins winter/summer US-Eastern boundaries. On top of that rule set, `strategy_reference_data/tests.rs::holiday_early_close_open_override_and_dst_use_exchange_local_dates` pins a published closure, half day and Sunday open override resolved in exchange-local dates and reported as `PublishedClosure` rather than a derived holiday; `one_published_close_minute_resolves_to_two_utc_instants_across_dst` proves one 13:00 local close lands at 17:00Z in EDT and 18:00Z in EST and that the fall-back repeat hour is one published date; `an_early_close_shortens_the_venues_own_windows_and_never_invents_an_open` proves the shortened day is the venue's own windows truncated — a policy-only calendar and a close at or before the first open are both refused rather than given an invented bell; `a_full_length_override_is_not_reported_as_an_early_close` keeps the session-relative flag honest. `incomplete_outage_rule_only_yahoo_and_dishonest_authority_fail_closed` proves an incomplete batch, an uncovered range, the built-in rule set, a mislabelled keyless feed and an unreachable source each fail closed. **What is still missing is the data**: no exchange or contracted-vendor feed is integrated, so an authoritative artifact can only be sealed from source records something else persisted |
| §6.8 corporate actions and adjustment consistency | **Foundation landed; materialization and native selection landed; automated fetchers open** | `split_then_dividend_adjusts_live_position_and_cash_in_canonical_order` pins effective-time split-before-dividend units, basis and cash. `verified_run_assembly_refuses_corporate_actions_already_baked_into_prices` proves the identity-bearing run boundary rejects double-counting against split-adjusted prices, and `adjusted_price_double_count_guard_rejects_splits_and_total_return_dividends` proves the same refusal at the ingestion boundary. `split_and_dividend_convert_safely_and_have_deterministic_order` pins split-before-dividend order and refuses `2.0` and `0.250` as second spellings; `source_ranks_match_the_schedule` pins the source rank table to the schedule's so records and sealed actions cannot sort apart; `ambiguous_and_non_canonical_timestamps_are_refused` rejects offsetless, local and `+00:00` stamps; `duplicate_conflict_unsupported_out_of_range_and_raw_tampering_are_rejected` and `a_repeated_calendar_date_is_a_duplicate_and_a_contradictory_one_is_a_conflict` name a redundant record apart from a contradictory one in both families. Fractional cash-in-lieu and the automated fetchers that would persist source records remain open |
| Reference-data identity, restart recovery and v3 compatibility | **Met for the boundary that exists** | `declaration_order_is_canonical_and_retrieval_time_is_audit_only` proves two declaration orders and two retrieval times seal one id while an as-of change does not; `a_decoded_artifact_whose_records_are_reordered_is_refused` and `bounded_codec_rejects_oversize_unknown_noncanonical_and_tampered_artifacts` pin bounded-before-decode, unknown-field, non-canonical-encoding and tampered-id refusals; `oversize_batches_and_raw_records_are_refused` pins the record and raw-byte bounds before any sort or hash. `restart_identity_and_v4_execution_binding_round_trip` stores both artifacts, reloads them from their ids alone, compares byte-identically, refuses a path used as an id, binds them into settings and re-derives the sealed config id. `v3_execution_configs_keep_their_sealed_identity` pins the pre-existing v3 vector as still loadable, verifiable and unchanged; `a_v3_config_may_not_carry_reference_data_bindings` and `unknown_execution_config_schema_versions_are_refused` close the relabelling and unknown-version paths; `reference_calendar_bindings_must_match_the_instrument_calendars` refuses a binding no instrument carries and a materialized calendar the bindings omit |
| Native reference-data inspection, materialization and selection | **Landed for the local-snapshot path** | `strategy_reference_data_worker/tests.rs::inspecting_a_snapshot_reports_authority_coverage_and_never_promotes` proves inspection runs off the submitting thread, reports authority/coverage/completeness, and leaves the store empty; `rule_only_keyless_and_unreachable_sources_are_blocked_with_a_reason` proves each non-authoritative class is reported blocked *with the refusal* and then actually fails to materialize; `materializing_then_selecting_seals_a_config_and_labels_its_authority` seals both artifacts, lists them, binds them and rebuilds the same v4 config id while labelling the pair non-authoritative because one source is a cache of a keyless feed; `the_job_queue_is_bounded_and_reports_backpressure` pins non-blocking backpressure, the per-poll ceiling and a terminating drop of a backlogged worker; `a_listing_bounded_below_the_store_reports_what_it_omitted` proves a truncated listing counts what it hid; `unreadable_snapshots_and_unknown_artifact_ids_fail_without_stopping_the_worker` covers missing, garbage, directory and path-as-id inputs. The panel view model is pure and separately tested in `typhoon-native/src/app/reference_data_model/tests.rs`: no defaults, promotion gated on the worker's own dry run, superseded replies dropped, slot changes discarding a stale preparation, vanished artifacts clearing their slot, omissions surfaced, and backpressure freeing the pending slot. Sealing a source below exchange/vendor authority additionally requires an explicit operator acknowledgement that starts false and is cleared by every new inspection (`a_non_authoritative_source_needs_an_explicit_acknowledgement`), so a durable artifact from a rule-derived or keyless feed is a stated decision rather than a default. Snapshot selection uses the platform file dialog, which yields a path only — the file is opened, bounded and decoded on the worker |
| §6.9 sub-bar fidelity | **Identity-bound engine and native single-run path landed; tick replay open** | Run-manifest schema v5 content-addresses one finer immutable dataset per parent input. Verified assembly checks the dataset seal, exact id, symbol, adjustment/calendar policy and declared finer fixed timeframe before simulation constructs a bounded path. `verified_sub_bar_resolution_*` covers success plus missing, unexpected, mismatched, tampered and incompatible inputs; `sub_bar_path_requires_exact_causal_tiling_without_gaps_or_overlaps` rejects duplicate paths, gaps, overlaps, out-of-parent bars and incomplete coverage, while `sub_bar_fidelity_uses_the_earlier_path_step_before_parent_ambiguity` proves causal event ordering. The native Backtest Engine selects bounded parent/finer summaries from the Dataset Inspector plus existing sealed strategy/config/manifest JSON, snapshots chart identity and timeline generation, submits one bounded worker job, ignores stale completions, and installs only a matching prepared overlay/report. It does not generate artifacts. True tick fidelity remains unavailable because TyphooN retains no versioned tick history |
| §6.3 deferred financing/borrow/funding/currency costs | **Foundation landed; paid/live inputs unavailable fail closed** | `financing_uses_last_committed_mark_and_reconciles_report_totals` pins the committed mark, interval, financing/funding debits, cash and report total; `strategy_financing/tests.rs::accrual_uses_declared_units_and_refuses_missing_borrow` proves unavailable short borrow is an error, not zero. Current rates/conversions are constant, provenance-bearing assumptions; no paid/live historical rate feed is integrated |

Remaining M2 packet: the ingestion, materialization, identity and native selection half of
authoritative calendars and corporate actions is now landed and tested, but **no exchange or
contracted-vendor feed is integrated**, so in practice only `UnverifiedPublic` and `DerivedRule`
source records exist to seal — and neither may back a run that requires authority. Acquiring
those feeds, plus entitled historical/live borrow, financing, funding and currency inputs, is
what remains. Until those paid/live inputs exist, unavailable rates and non-authoritative
reference data continue to fail closed rather than being substituted. Multi-symbol
corporate-action schedules in one config, automated fetchers that persist source snapshots, and
fractional cash-in-lieu are also open. True tick fidelity remains unavailable until TyphooN
retains an immutable, versioned tick-history corpus. **M2 is not complete.**

**M3–M8:** no later milestone gate has passed. Their remaining work is exactly the delivery and
gate text below; they may now build on the completed M1 correctness foundation.

### M0 — Dataset foundation & QA — **gate passed 2026-07-27** (see §13.1)
**Prereqs:** none (builds on the existing cache/merge stack).
**Delivers:** immutable content-addressed datasets + manifest + provenance (§5.1); dataset
QA pass and report (§11.1); tabular inspector (§11.2); versioned dataset-side calendar
identity/policy and session-aware weekend-candle diagnostic (§11.5).
**Gate:** a dataset id reproducibly materializes byte-identical bars across restarts; QA
detects seeded synthetic defects (gap, spike, duplicate timestamp, carry bar, OHLC
violation, split-like level shift, unexpected weekend/session bar) at 100 % on the test
corpus; building a dataset never blocks the render thread.

### M1 — Simulation correctness (**gate passed 2026-07-27; the hard gate**)
**Prereqs:** M0.
**Delivers:** strategy IR v1 + reference interpreter (§5.2); event-driven simulator (§5.3);
the M1 order, fill, per-trade cost, latency and OHLC mechanics from §6.1–§6.5, plus deterministic synchronization/no-look-ahead semantics
§6.10–§6.12; core closed-bar/next-open/pre-close decision timing and forming-bar visibility
from §6.13; cost models for Kraken + Alpaca; run manifests.
Time-accrued financing, borrow, crypto funding and currency conversion, plus strategy-authored
protective-management templates, are explicitly assigned to M2's richer-execution/two-leg slice;
they are not silently treated as zero-cost M1 behavior.
**Gate — all of the following, or M1 is not done:**
1. **Golden tests**: a corpus of hand-computed scenarios (single long trade with commission
   and spread; stop-and-target-in-the-same-bar under each OHLC policy; gap through a stop;
   limit order that never fills; reversal with costs; warm-up boundary) where every expected
   trade, fill price, fee and equity value is derived by hand and asserted exactly.
2. **Determinism**: identical inputs ⇒ bit-identical outputs, verified in CI, including
   under multi-threaded execution.
3. **Look-ahead canary suite**: every cheating strategy fails to compile or trips the guard,
   including a pre-close strategy that tries to read the forming bar's final OHLC and a
   multi-timeframe strategy that tries to read a higher-timeframe bar before it closes.
4. **Zero-cost equivalence**: with all costs, latency and slippage set to zero and
   bar-close fidelity selected, the new engine reproduces the legacy `run_backtest` results
   for the five existing strategies — proving the difference is the *model*, not a
   regression.
5. **Cost sensitivity is visible**: the same strategy at 0×/1×/2× costs produces materially
   different, correctly-ordered results.

**No broad generation, optimization, or robustness work starts before this gate passes.**
This is the central sequencing decision of this ADR: the current tooling's most dangerous
property is that it produces confident numbers from an unrealistic model, and scaling that
up multiplies the error rather than the insight.

### M2 — Metrics, reporting, and richer execution
**Prereqs:** M1.
**Delivers:** versioned metric registry (§9) incl. MAE/MFE, calendar equity, exposure,
stagnation, tails and stability; report + manifest export; execution realism §6.6–§6.9
(partial fills, sessions/timezones, corporate actions, sub-bar fidelity); chart trade
overlay; deterministic visual/manual/hybrid replay using the M1 timing semantics (§6.13);
repainting diagnostic (§11.5); NNFX two-leg order lifecycle (§10.3).
**Gate:** every metric has a written definition and a hand-verified golden test; no metric
returns a magic sentinel; calendar equity reconciles exactly with the trade list; MAE/MFE
verified against hand-computed excursions; reports round-trip through JSON without loss.
Replaying a recorded hybrid run reproduces its ledger bit-for-bit, and a synthetic repainting
indicator is identified at the exact mutated bar/output. A hand-computed two-leg scenario
asserts each entry, independent target/stop, break-even transition, trailing step, fee, and
ledger/equity effect exactly before portfolio-level money-management work begins.

### M3 — Builder & databank
**Prereqs:** M1, M2.
**Delivers:** visual IR builder (§5.2); databank schema + indexed queries (§5.7); results
browser with filtering/sorting/comparison; strategy identity + canonical hashing; guided NNFX
role/profile builder with entry/rule toggles and long/short run constraints (§4.8, §5.11).
**Gate:** a strategy built in the GUI, saved, reloaded and re-run reproduces its stored
metrics exactly; two structurally identical strategies collapse to one id; databank queries
over a synthetic 10⁵-run corpus stay bounded and off the render thread. Every guided NNFX
profile/slot/rule configuration round-trips through the canonical IR and produces the same IR
as the equivalent general-builder graph.

### M4 — Optimizer, retester & robustness pipeline
**Prereqs:** M1–M3.
**Delivers:** general parameter optimization over the IR (grid/random/local/Bayesian);
retester; OOS schemes (§7.1); walk-forward incl. optimization and matrix (§7.2); Monte Carlo
families (§7.3); plateau/SPP/optimization profile (§7.4); cross-checks (§7.5); degradation
gates and problem recognition (§7.6); multiple-testing controls (§7.7); holdout/quarantine
enforcement (§7.8); 3D and parallel-coordinates views.
**Gate:** a deliberately curve-fit strategy (fit to a known-random synthetic series) is
**rejected** by the pipeline; a synthetic strategy with a *planted*, genuine edge
**survives**; holdout access is refused by the API from within search stages; every reported
"best" displays its N.

### M5 — Generation
**Prereqs:** M4 (robustness must exist before mass generation, or the databank fills with
noise).
**Delivers:** typed grammar sampling, templates/placeholders, constraints (§8.1);
evolutionary + random/Bayesian search (§8.2); novelty/dedup (§8.3); complexity penalties
(§8.5); deterministic resumability (§8.6); Candidate Search as constrained slot/template
enumeration over the same machinery.
**Gate:** a generation run is bit-reproducible from its seed; a killed-and-resumed run
matches an uninterrupted one; dedup demonstrably collapses known-equivalent candidates;
generated candidates are all type-correct and pass the look-ahead canary by construction;
random-search baseline is reported alongside evolutionary results (no unfalsifiable
"our search is smarter" claim).

### M6 — Portfolio, money management, improver, extensions
**Prereqs:** M4 (M5 recommended).
**Delivers:** multi-strategy portfolio simulation with shared capital/margin and concurrent
fills (§10.1, §10.3); fit-to-portfolio selection (§10.2); portfolio Monte Carlo, what-if,
equity control (§10.4); two-leg/ATR/break-even/trailing-stop money management and
asset/currency overexposure policies (§10.3); strategy improver (§8.4); extension traits
(§5.10).
**Gate:** portfolio equity from a combined run differs from naive PnL summation in a
capital-constrained scenario **by the hand-computed amount**; correlated-strategy selection
provably prefers a diversified set over a top-N-by-metric set on a constructed case. Golden
tests cover two-leg break-even/trailing transitions and each overexposure action (allow,
block, reduce risk) across multiple instruments/accounts, proving the M2 single-strategy
lifecycle remains correct under shared capital and account-level exposure policy.

### M7 — Workflows & automation
**Prereqs:** M4–M6.
**Delivers:** typed workflow DAG, artifact caching, resume, scheduling, unattended runs
(§5.9); tick fidelity if and when tick data exists (§6.9 level 4, §11.3).
**Gate:** a full generate → simulate → filter → robustness → portfolio-fit → store workflow
runs unattended to completion, is resumable after a kill at any stage, and re-running with
one changed stage recomputes only the affected subtree.

### M8 — Lifecycle & live feedback
**Prereqs:** M6, M7.
**Delivers:** strategy lifecycle states, paper/shadow validation, live-vs-simulated
divergence measurement feeding back into cost models, scheduled re-validation, strategy
signals/notifications, assistant mode, and one immutable configuration across backtest →
paper → explicitly enabled live operation (§10.5).
**Gate:** a paper-shadowed strategy reports measured fill/slippage divergence vs its
simulation, and that measurement demonstrably updates the cost model used by subsequent
backtests. The same strategy/config hash appears from backtest through paper/live eligibility,
and no strategy can enter automatic live mode without an explicit GUI action and audit record.

---

## 14. Non-goals, deferred, and blocked

**Non-goals (will not be built):**
- UI/visual cloning of StrategyQuant X or NNFX Algo Tester.
- Reimplementation or reverse-engineering of proprietary vendor algorithms.
- Code export to MT4/MT5/cTrader/NinjaTrader or any external platform — removed by
  [ADR-111](111-broker-scope-reduction-kraken-alpaca-only.md); TyphooN trades natively on
  Kraken + Alpaca.
- Loading MT4 `.ex4` binaries. Custom-indicator capability is provided through native Rust
  implementations and the deterministic `typhoon-transpiler` boundary.
- Cloud/SaaS execution, licensing servers, marketplaces, or multi-user collaboration.
- A general embedded scripting runtime (sandboxing/determinism cost not yet justified;
  extensions go through Rust traits or `typhoon-transpiler`).
- Martingale/position-escalation money management on any promotable path
  ([ADR-114](114-deprecate-martingale-live-trading.md)).
- Broker/asset-class expansion to reach vendor market coverage.

**Deferred (wanted, scheduled after prerequisites):**
- Fuzzy-logic grammar blocks (§8.1) — after core grammar stability.
- Custom data import (§11.4) — after the dataset abstraction settles.
- ML surrogate pre-screening in the generator (§8.2) — only with measured benefit.
- Distributed/multi-machine compute — after single-machine bounded execution is solid.

**Blocked (external dependency, honestly labelled):**
- **Real-tick backtesting** (§6.9 level 4) — blocked on a retained, versioned tick corpus.
  Source depth/entitlement still needs verification; forward-recording live L1/L2 is the
  known in-scope unblocking path without adding a paid dependency.
- **Borrow-cost realism for short equity** (§6.3) — borrow-rate feeds are paid-only, already
  noted as deferred in [ADR-120](120-regulatory-outlier-alerts.md); shorts simulate with a
  configured assumed rate that is **stamped on the report as an assumption**.
- **Depth-aware fills** beyond the venues/symbols where L2 is actually available
  ([ADR-129](129-l1-l2-l3-market-data-support.md)).

---

## 15. Relationship to ADR-038 (and other ADRs)

[ADR-038 — GPU Strategy Optimizer & MQL5 Export Pipeline](038-gpu-strategy-optimizer-and-mql5-export-pipeline.md)
is the **narrower historical predecessor** of this program: a GPU parameter-sweep optimizer
whose validation strategy terminated in MetaTrader 5. **It is retained for historical
context and is not to be deleted.** This ADR governs the broad program; where the two
conflict, **ADR-135 wins**.

Corrections to ADR-038's stale claims are summarized here and reflected in ADR-038's
current-status banner while its original proposal remains available as historical context:

| ADR-038 claim | Correction (2026-07-27) |
| --- | --- |
| **Status: Implemented** | Overstated for the program as a whole. What exists is a fixed SMA/NNFX GPU sweep plus a first-draft CPU backtester (§1.1). The Strategy DSL, visual builder, portfolio optimization and multi-objective work of its Phases 3 and 5 are **not** implemented. |
| Speedup table (600×, 5,760×, 3,000× vs MT5) | **Unsourced and unreproducible.** No measurement method, hardware, workload or MT5 configuration is recorded, and MT5 has since been removed from the project ([ADR-111](111-broker-scope-reduction-kraken-alpaca-only.md)) so the comparison can no longer even be run. **Treat these numbers as void.** This ADR makes no performance claims (§12.3). |
| "Phase 3: Strategy DSL — *Implemented via MQL5 compiler*" | The transpiler is a **source-language front-end**, not the strategy IR this program requires (§5.2). A parser for another vendor's language is not a typed, versioned, look-ahead-safe strategy representation. |
| "Phase 4: MQL5 Export — Implemented" | Removed with the MT5 scope reduction ([ADR-111](111-broker-scope-reduction-kraken-alpaca-only.md)). Export is now a **non-goal** (§14). |
| Step 6 "MT5 Final Validation" as the execution-realism backstop | **Gone, and never replaced.** Its removal is the reason execution realism must live *inside* TyphooN (§6) and be closed by paper/shadow validation against Kraken/Alpaca (§10.5). |
| "Built-in robustness analysis" (Phase 2 checkboxes) | Fixed walk-forward and neighbour-stability shader/pipeline objects exist, but no host dispatch API wires either one into a general robustness workflow. The wired Monte Carlo routine is daily-return portfolio VaR, not trade shuffling. These are **foundations, not a pipeline**: they are not composable, gated, stored, or applied to arbitrary strategies (§4.4). |
| Mitigation: "f32 vs f64 — final MT5 validation catches divergence" | **Invalid** — that validation step no longer exists. Replaced by mandatory CPU-reference re-verification of GPU-selected candidates (§12.3). |
| Mitigation: "Execution effects: GPU finds the region, MT5 validates realism" | **Invalid** for the same reason. Execution realism is now an in-engine requirement (§6). |

**Other ADR relationships:**
- **Constrains / respects:** [ADR-111](111-broker-scope-reduction-kraken-alpaca-only.md)
  (Kraken + Alpaca only; no MT5/export re-entry),
  [ADR-114](114-deprecate-martingale-live-trading.md) (no martingale),
  [ADR-115](115-deprecate-cli-tui.md) (native GUI is the product surface),
  [ADR-126](126-primary-assist-broker-selection.md) /
  [ADR-130](130-multi-account-broker-support.md) (order routing and account model),
  [ADR-133](133-command-palette-research-only.md) (palette stays research-only; strategy
  controls are graphical).
- **Builds on:** [ADR-003](003-sqlite-bar-cache.md) (bar cache),
  [ADR-112](112-equities-bar-sync-demand-depth-vs-catalog-breadth.md) /
  [ADR-113](113-cross-source-equity-bar-merge-data-integrity.md) /
  [ADR-122](122-curated-stock-split-fallback-for-equity-merge.md) /
  [ADR-124](124-depth-era-promotion-must-not-redefine-price-scale.md) (data integrity and
  provenance), [ADR-110](110-market-session-status-xstocks-24-5-and-us-equities.md)
  (sessions), [ADR-119](119-live-forming-bar-overlay-source-policy.md) (forming-bar
  semantics), [ADR-129](129-l1-l2-l3-market-data-support.md) (depth data).
- **Bound by:** [ADR-098](098-per-frame-o-1-discipline-in-chart-and-sync-paths.md) and
  [ADR-134](134-render-independent-background-pump.md) (nothing heavy on the render thread),
  [ADR-118](118-test-module-decomposition-convention.md) (test module layout),
  [ADR-125](125-native-crate-boundary-plan.md) /
  [ADR-086](086-typhoon-native-app-rs-module-decomposition-for-compile-speed.md) (crate and
  module boundaries — the simulation core belongs in `typhoon-engine`, the GUI in
  `typhoon-native`).
- **Uses:** [ADR-040](040-typhoon-transpiler-pipeline-source-to-gpu-cpu-execution.md) /
  [ADR-067](067-multi-frontend-expansion-cross-language-transpiler.md) (transpiler as an IR
  front-end), [ADR-001](001-native-gpu-architecture.md) /
  [ADR-030](030-gpu-compute-architecture-wgpu-compute-shaders-for-all-numerical-work.md)
  (GPU infrastructure, subject to §12.3).

---

## 16. Consequences

**Positive**
- The project stops reporting first-draft backtest numbers as if they were decision-grade.
- A versioned IR + deterministic simulator makes every result reproducible and auditable —
  the property that actually distinguishes a research platform from a demo.
- Robustness, portfolio and workflow capability become *compositions* of one correct core
  rather than a growing set of disconnected windows.
- NNFX users get a focused guided path without trapping the project in a second strategy
  representation: the form, visual graph, optimizer, replay, and live lifecycle all share
  one canonical IR/configuration.
- TyphooN's multi-source data stack becomes a genuine differentiator: cross-source
  robustness checks (§7.5) are something a single-vendor data pipeline cannot do.
- Non-goals are written down, so scope creep toward export/cloud/broker-matrix parity has a
  document to lose to.

**Negative / costs**
- M1 replaces the current simulation core. The five existing strategies must be re-expressed
  in the IR, and old result screenshots/numbers become non-comparable.
- The realistic engine will report **worse** results than the current one for the same
  strategies. That is the point, and it must be communicated as a fix, not a regression.
- Significant new surface: IR, simulator, databank, workflow engine — all needing tests.
- The GPU path becomes *subordinate* to a CPU reference, which costs some of the speed
  advantage that motivated ADR-038.

**Risks**
| Risk | Mitigation |
| --- | --- |
| Scope collapse — the program stalls after M1 and leaves two half-engines | M1's gate includes zero-cost equivalence with the legacy engine, so the old path can be retired immediately on completion |
| Over-engineering the IR before real strategies exercise it | IR v1 is scoped to express the five existing strategies plus stops/targets/sizing; extend only under demand |
| NNFX parity turns into hard-coded special-case logic | Guided profiles lower into and round-trip through the canonical IR; M3 rejects any profile that cannot be expressed equivalently in the general builder |
| Robustness theatre — many statistics, no discipline | Gates produce *verdicts* with pass/fail thresholds and stored evidence, and the holdout is API-enforced (§7.8) |
| Curve-fitting at industrial scale once generation lands | M5 is gated behind M4; multiple-testing controls (§7.7) and SPP (§7.4) are mandatory, not optional views |
| Determinism erosion as parallelism is added | Bit-identical CI test on every change to the simulation path |
| Cost-model wrongness invalidating everything downstream | M8's live-vs-simulated divergence feedback measures it directly against the venues we trade |
| UI stalls from databank growth | Indexed bounded queries + off-render-thread execution (§12.2) |
| Metric drift making historical runs incomparable | `metrics_version` + `engine_version` stamped on every run; comparisons across versions are labelled |

---

## Primary sources

Vendor marketing pages, accessed **2026-07-27**. Cited for *publicly advertised capability*
only; no claim is made about internal implementation.

- StrategyQuant X — features (incl. robustness section): <https://strategyquant.com/features/>
- QuantAnalyzer: <https://strategyquant.com/quantanalyzer/>
- QuantDataManager: <https://strategyquant.com/quantdatamanager/>
- NNFX Algo Tester — product features: <https://nnfxalgotester.com/#Features>
- NNFX Algo Tester Help — complete feature list: <https://help.nnfxalgotester.com/knowledgebase.php?article=115>
- NNFX operation modes and repaint/weekend diagnostics: <https://help.nnfxalgotester.com/knowledgebase.php?article=86>
- NNFX Candidate Search: <https://help.nnfxalgotester.com/knowledgebase.php?article=113>
- NNFX money management: <https://help.nnfxalgotester.com/knowledgebase.php?article=56>
- NNFX overexposure management: <https://help.nnfxalgotester.com/knowledgebase.php?article=110>
- NNFX algorithm configuration: <https://help.nnfxalgotester.com/knowledgebase.php?article=52>

Internal evidence, read **2026-07-27** at commit `f9d383e9`:

- `typhoon-engine/src/core/backtest.rs` (1,183 lines) — `Strategy` trait, five strategies,
  `run_backtest`, `bar_by_bar_backtest`, `optimize_sma_cross`, `walk_forward`, `TradeReport`.
- `typhoon-engine/src/core/backtest/tests.rs` — 53 unit tests.
- `typhoon-native/src/app/strategy_windows.rs` (926 lines) — backtest window, optimizer
  window (CPU + GPU SMA/NNFX), Fast × Slow Sharpe heatmap, walk-forward summary.
- `typhoon-native/src/gpu_compute` — `ParamCombo`, `NnfxParamCombo`, fixed WGSL shaders.

## Open questions

1. **IR expressiveness ceiling for v1** — does v1 include multi-timeframe and multi-symbol
   node types, or are those an explicit v2 extension gated on M4?
2. **Default cost model per venue** — Kraken and Alpaca fee schedules are tiered and change;
   where does the canonical schedule live, and how is it versioned so old runs stay
   reproducible?
3. **Databank storage location** — same SQLite file as the potentially large,
   contention-sensitive bar cache, or a separate database? A separate file is the probable
   answer, pending measurement.
4. **Holdout definition granularity** — per dataset, per symbol, or per research programme?
5. **Forward tick recording** — is recording live L1/L2 to disk worth the storage cost now,
   given tick-mode backtesting otherwise remains blocked?
