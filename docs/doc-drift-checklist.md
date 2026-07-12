# Doc Drift Checklist (Living)

**Purpose**: Track mismatches between current implementation (code + runtime behavior) and documentation.  
**Style**: Prefer semantic names over old "feature parity" sequencing. Update on changes.  
**Maintenance**: Before major work, run searches for "stub|pending|not yet|parity|future|missing" in docs/ + cross-check key code areas. Mark items [x] when fixed and re-commit.  
**Last full sweep**: 2026-07-12 (implementation-alignment comb-over: performance telemetry, bounded background snapshots, single-owner session persistence, broker-refill backpressure, current compression policy, and Kraken-equities lane semantics). ADR-130 closed (Kraken private-WS follows primary switch via kraken_private_ws_task abort+respawn; KrakenTradeCopy one-shot xStock spot replication with margin-skip + catalog doomed-order guard; broker-aware TradeCopy window). ADR-120 SSR shipped (computed Rule-201 state machine: engine trigger/expiry/purge + native 30s watchlist scan, holiday-aware expiry). ADR-117 closed (keyless Reddit mention lane + research_social_history sparkline in the SENTIMENT window; sentiment-v2 superseded by local history). ADR-116 all eight TODOs closed (FinvizSnapshot + packet section; perf windows; derived ratios/growth; employees ingested + optionable inferred; headline day-impact; MARKET_MAP treemap + sector groups; ScreenerField registry + saved screens). ADR-084 max pain shipped (engine + packet line). ADR-113 live-tick anchor shipped (narrow newest-bar clamp vs fresh real-time quote at merged installs). ADR-048 #7 cross-TF drawings assessed and deliberately kept open (bar-index coordinate model across 89 Drawing variants + persisted-session migration = dedicated pass). Externally blocked, documented as such: ADR-120 delisting feed (no machine-readable source) + borrow rates (paid-only), ADR-110 iapi schedule endpoint items (undocumented endpoint). Prior sweep 2026-07-04 (ADR deferred-work completion pass).

**Status legend**:
- [ ] Open drift (code ahead or doc outdated)
- [x] Fixed / in sync
- [~] Acceptable historical (intentional record, git-history pointer, or ADR title)

**Latest full comb-over (2026-07-12 implementation alignment):**
- Counted 110 ADR files and verified the six Cargo workspace packages against `cargo metadata`.
- Cross-checked performance/sync/storage ADRs against `app_runtime.rs`, `app_background.rs`, `app_runtime_broker_messages.rs`, `market_data_sync.rs`, session persistence, and `typhoon-engine::core::cache`.
- **Drift fixed:**
  - Replaced unsupported fixed startup/render/memory benchmark claims with the phase-attributed telemetry actually emitted by the runtime.
  - Corrected the chart data-flow description: TTBR is decoded into owned bars; JSON still exists at provider boundaries, so “zero serialization everywhere” was false.
  - Restored ADR-032/033’s current capacity-one `BgData` snapshot channel, 3s lightweight/5m full cadence, nonblocking publication, and off-thread destruction. Documented the 45+ GB high-water failure that exposed the accidental unbounded channel.
  - Recorded single-owner incremental session persistence and removal of duplicate minute-cadence session JSON/preferences work.
  - Recorded saturated heavy-sync broker-refill deferral (`pending > 200`) while preserving periodic full-universe progress.
  - Corrected ADR-089/099 and high-level storage docs: normal bars, `merge_bars_fast`, and current KV writes honor the configured zstd level; WS no longer silently forces level 3.
  - Corrected Kraken equities scope across ROADMAP/PERFORMANCE/ARCHITECTURE and ADR-101/102: iapi is demand-depth repair; the June 8 full-catalog experiment was reversed by ADR-112; bounded WS snapshot and Alpaca/Yahoo merged lanes own catalog breadth.
  - Corrected ADR count 109→110 in the architecture tree and removed stale CryptoCompare union wording from the active Kraken roadmap.
  - Marked ADR-037/066/069/131 statuses honestly; updated ADR-110 and ADR-115 to current implementation/workspace state; clarified ADR-103 and ADR-088 historical provider/path references.
  - Updated README volatile metrics, alert delivery, broker-adapter scope, transpiler description, and ADR index annotations; updated research-packet AI transports and removed brittle fixed sub-block counts.
  - Revised ADR-112/128 for bounded full-universe native WS coverage from `1Week` through `1Min` while retaining demand-scoped iapi depth repair and derived `1Month`.
- Historical removed-system sections remain explicitly labeled as history rather than rewritten as current architecture.
- The prior 2026-07-08 command/palette audit remains valid; no command-surface changes were made in this pass.

## 1. High-level Docs (ROADMAP.md, ARCHITECTURE.md, DESIGN_PHILOSOPHY.md, INDICATORS.md)

- [x] Security-first dependency refresh (2026-07-12): all workspace manifests
  share common direct requirements, explicit minimum feature sets replace broad
  defaults, `tokio-tungstenite` is current, the lockfile is at its compatible
  ceiling, RustSec is clean under documented policy, and every remaining
  duplicate family is traced to an upstream owner — ADR-031/088.
- [x] Broker scope (Kraken Spot / Equities/xStocks / Futures + Alpaca + Yahoo corroborator) — matches code + tiers in ARCHITECTURE/PERFORMANCE.
- [x] Deprecations (CLI/TUI archived on deprecated/cli-tui, MT5/Darwinex, CryptoCompare, LAN, martingale, custom TFs) — correctly documented in ROADMAP and DESIGN_PHILOSOPHY.
- [x] Native GUI primary / engine as library — accurate.
- [x] "Parity" terminology — softened in DESIGN_PHILOSOPHY (NNFX System Equivalence, computational equivalence), ROADMAP (native implementation target, Research and indicator surfaces), INDICATORS.md (MT5-style / computational match), ARCHITECTURE (GPU/CPU equivalence required). Historical ADR titles left as-is.
- [x] Recent market data (L1 sizes, Kraken L2 CRC/v2 book, L3 foundation, depth profile, richer Bookmap) — added to ARCHITECTURE + ROADMAP + DESIGN_PHILOSOPHY in 2026-07 sweep.
- [x] MTF Grid, depth profile, Bookmap — added explicit bullets to ROADMAP Phase 4 (floating windows + depth profile overlay).
- [x] Symbol Explorer described as catalog browser (not full scanner) — still accurate in ARCHITECTURE.
- [x] Research packet / indicator surfaces — matches code.
- [x] ARCHITECTURE Project Structure lists the 6-crate workspace (added `typhoon-broker-runtime` / `typhoon-chart-ui` / `typhoon-research-ui` per ADR-125 Complete) + engine `broker/protocol.rs` + `capabilities.rs`; ADR count corrected 115→106 (2026-07-02).
- [x] docs/adr/README.md ADR count corrected 120→106; recent ADRs (121–124, 127–129) added to thematic groups; README ADR index extended 126→129 (2026-07-02).
- [x] RESEARCH_PACKET.md: dropped removed `DARWINEX`/`TASTY` scope labels, fixed D1 bar-key probe (`mt5:`→`kraken-equities:`), and empty-cache/data-source hints (MT5SYNC→BARDATA) per ADR-111 scope reduction (2026-07-02).
- [x] Console / palette command counts and examples — 2026-07-08 sweep fixed stale 225, CONNECT, OPTIMIZER desc, POSITION_CHARTS across README/ROADMAP/ARCHITECTURE/DESIGN/KEYBOARD. Registry-driven reality documented.

## 2. Broker & Market Data (L1/L2/L3, Sync, Tiers)

- [x] Multi-account brokers (ADR-130, clarified 2026-07-12): Alpaca 4-slot account pool with successfully-connected-only round-robin at request/batch dispatch, shared canonical cache keys, aggregate capacity scaling, normal settlement/retry re-entry, and Primary-independent bar routing; Kraken extra identities do not add data capacity. Account-granular Primary, TradeCopy, Sync Status disabled-TF honesty, and fills refresh are documented in ADR-130 + API_KEYS + README + ROADMAP + ARCHITECTURE + PERFORMANCE.
- [x] Uniform Alpaca slots: Key/Secret/Paper|Live only; specs enable sync/trade but runtime participation requires successful connection. Credentials persist on edit; TradeCopy/mirroring remains explicitly opt-in and non-persistent.
- [x] L3 foundation (ws_v2_level3.rs, streamer with token/real/sim, CRC32 apply_with_checksum, KrakenL3State, received_at_ms, per-order, token/no-token entitlement status, projection to same paths, Bookmap/depth integration, tests) — covered in ADR-129 + ADR-109 Update sections.
- [x] L2 v2 book (CRC, exact tokens, 25 levels, shared DOM depth preference across user-facing stream entrypoints, v1 kept for compat) — documented; historical "pending" status lines in ADR-109 cleaned in 2026-07.
- [x] L1 (ticker/quotes/public trades with sizes, O(1) dispatch, watchlist/chart freshness parity) — consistent.
- [x] Kraken Futures bars/sync — present in code (bar_fetch) and docs.
- [x] Alpaca as assist/fallback + trading (tier-autotuning, catalog-breadth) — matches ADRs 087/112/113.
- [x] M1/M5 rules (Kraken Spot/Equities valid low-TF targets; assist rows non-target) — live public trades WS + forming vol + WS-fresh + sync priority now wired for low-TF MTF. Referenced + implemented in code paths.
- [x] Sync schedulers / O(1) / coverage-first / AIMD / pending-work (ADRs 029, 087, 094, 098, 102, 107, 128) — language generally aligns with current bounded queues, backfill-complete markers, etc.
- [~] Many "pending work", "missing bars", "gap fill" references in sync ADRs — mostly accurate descriptions of ongoing mechanisms, not outdated claims.
- [x] v1 public_book.rs kept for legacy/compat — correctly noted in ADR-109.
- [x] Typed broker capability model (`typhoon-engine::broker::capabilities` — `MarketDataSupport`/`DepthAssetScope`/`BrokerMarketDataCapabilities`, exhaustive over `OrderBroker`) + typed `MarketDataProvenance`/`MarketDataTransport` — documented in ADR-129 ("Broker Capability Model (code)") and reflected in ARCHITECTURE data-sources note (2026-07-02).
- [x] Self-healing reconnect + heartbeat half-open watchdog across all four Kraken WS lanes (ticker/book/trade/level3) — documented in ADR-129 reconnect/half-open bullets (2026-07-02).

## 3. Charts, UI & Visualization

- [x] Depth profile (live bins from 25 levels, "L3 depth" label + tint distinction, overlay) — in code (render.rs, app_runtime), now referenced in ARCHITECTURE + ADR-129.
- [x] Bookmap richer (per-order markers, scroll list, age coloring via received_at_ms + timestamp, interactions/click/copy, is_l3 detection) — covered in ADRs and recent updates.
- [x] MTF Grid parity (depth/L3 updates via chart_by_bare) — noted in ADR-129.
- [x] Drawing tools count (89 in ROADMAP/ARCHITECTURE/ADR-048) — verified consistent; bodies in ADR-048 softened to TradingView-style.
- [x] 46+ indicators claim in ROADMAP — verified consistent across ROADMAP, INDICATORS.md, ARCHITECTURE, PERFORMANCE (exact "46+" used as approximate).

(continues with prior sections on AI, commands, etc. — see previous entries for broker/console specifics now updated in this sweep)

## How to Use This Checklist
1. Run: `grep -rE 'stub|pending|not yet|parity|future work|in progress' docs/ --include="*.md" | head -30`
2. Cross-check code: `grep -r \"depth_profile\\|L3\\|Bookmap\\|received_at_ms\\|chart_by_bare\" --include=\"*.rs\" typhoon-native/src/app/ typhoon-chart-ui/ | head -10`
3. Update this file, apply fixes, commit with "docs: drift checklist updates".
4. Prefer small coherent PRs per section.

**Next sweep triggers**: New L3 features, broker changes, UI panels, performance work, any removal.

**Items fixed in this sweep** (see git history for details):
- Generated living docs/doc-drift-checklist.md
- ADR-109 phase statuses
- ARCHITECTURE recent market data note
- Softened parity terminology across DESIGN_PHILOSOPHY / ROADMAP / INDICATORS.md / ARCHITECTURE
- Added depth profile + Bookmap coverage to ROADMAP and DESIGN_PHILOSOPHY
- Updated ADRs 027/017/004/005/048/069 with modern L3/depth/Bookmap/MTF propagation and softened language
- Added M1/M5 low-TF rules to ARCHITECTURE
- Clarified Kraken Futures (data-focused) in API_KEYS.md
- Verified drawing tools (89) and indicators (46+) counts
- All remaining open drift items from checklist tackled
- **2026-07-08 console/command/ADR pass**: 225 counts, CONNECT (non-existent), OPTIMIZER desc, POSITION_CHARTS (historical note), LAN remote cmd claims, palette registry reality vs docs; ADR-082/130 accuracy confirmed; unregistered AI console cmds noted.

---

*This is a living document. Treat it as the source of truth for doc maintenance priorities.*
