# Doc Drift Checklist (Living)

**Purpose**: Track mismatches between current implementation (code + runtime behavior) and documentation.  
**Style**: Prefer semantic names over old "feature parity" sequencing. Update on changes.  
**Maintenance**: Before major work, run searches for "stub|pending|not yet|parity|future|missing" in docs/ + cross-check key code areas. Mark items [x] when fixed and re-commit.  
**Last full sweep**: 2026-07-08 (this sweep: docs/ADRs console/command accuracy pass). ADR-130 closed (Kraken private-WS follows primary switch via kraken_private_ws_task abort+respawn; KrakenTradeCopy one-shot xStock spot replication with margin-skip + catalog doomed-order guard; broker-aware TradeCopy window). ADR-120 SSR shipped (computed Rule-201 state machine: engine trigger/expiry/purge + native 30s watchlist scan, holiday-aware expiry). ADR-117 closed (keyless Reddit mention lane + research_social_history sparkline in the SENTIMENT window; sentiment-v2 superseded by local history). ADR-116 all eight TODOs closed (FinvizSnapshot + packet section; perf windows; derived ratios/growth; employees ingested + optionable inferred; headline day-impact; MARKET_MAP treemap + sector groups; ScreenerField registry + saved screens). ADR-084 max pain shipped (engine + packet line). ADR-113 live-tick anchor shipped (narrow newest-bar clamp vs fresh real-time quote at merged installs). ADR-048 #7 cross-TF drawings assessed and deliberately kept open (bar-index coordinate model across 89 Drawing variants + persisted-session migration = dedicated pass). Externally blocked, documented as such: ADR-120 delisting feed (no machine-readable source) + borrow rates (paid-only), ADR-110 iapi schedule endpoint items (undocumented endpoint). Prior sweep 2026-07-04 (ADR deferred-work completion pass).

**Status legend**:
- [ ] Open drift (code ahead or doc outdated)
- [x] Fixed / in sync
- [~] Acceptable historical (intentional record, git-history pointer, or ADR title)

**Latest full comb-over (2026-07-08 console/ADR accuracy):**
- Cross-checked docs/adr/*.md (110 files) + high-level docs (README, ROADMAP, ARCHITECTURE, DESIGN_PHILOSOPHY, KEYBOARD_SHORTCUTS, API_KEYS, RESEARCH_PACKET, doc-drift-checklist) vs live code: typhoon-native/src/app/commands.rs (228 Command {} entries), command_palette.rs (handle_command + special paths), command_palette/ai_commands.rs (ASK*/RESUME*/HERMES handlers + investigate_symbols + packet), ai_processes.rs (antigravity_cli_binary prefers "agy" > "antigravity" > "gemini"; spawn_*, model options).
- Command registry: research-only palette (ADR-133). Drawing/chart-type/indicator/timeframe/template/screenshot commands were removed from the registry and hidden handlers; fuzzy autocomplete from COMMANDS is for research/workflow commands only. Many AI/RESUME* research aliases remain supported via direct starts_with in handle_ai_command (called before handle_command registry matching) even if absent from registry.
- Verified: ASKANTIGRAVITY primary (with ASKGEMINI legacy), TRADECOPY|TRADE_COPY|COPYTRADE aliases, KRAKEN command (connect/balance/trade desc), INGEST_RESEARCH/BARDATA/AICACHE present, NEW_TAB/CLOSE_TAB present.
- **Drift fixed**:
  - Prior hard-coded command counts were superseded by ADR-133; docs now describe the palette by policy (research-only) instead of stale counts.
  - KEYBOARD_SHORTCUTS: removed non-existent `CONNECT` (replaced with real `KRAKEN`); fixed `OPTIMIZER` desc from "SMA Cross grid optimization" to "Strategy parameter optimizer" (matches commands.rs).
  - ROADMAP: updated Phase 4 console count; Phase 6 optimizer desc; Phase 10 changed POSITION_CHARTS [x] to historical note (absent from COMMANDS + no handler); Phase 14 LAN "15 remote commands (SEC_SCRAPE, FETCH_BARS...)" updated with accuracy note (LAN removed; current equivalents like INGEST_RESEARCH/BARDATA live in palette).
  - Confirmed no CONNECT or POSITION_CHARTS in code/handlers.
- **ADRs vs impl**:
  - ADR-082 (AI chat persistence + resume slash commands): accurate for its date (RESUMECLAUDE/ANTIGRAVITY/CODEX/AI + AISESSIONS; agy/antigravity/gemini detection; ASKANTIGRAVITY primary + ASKGEMINI legacy). Post-ADR additions (ASKGROK, ASKHERMES, ASKAI multi-provider, Codex reasoning) not claimed as "all" at time; current code matches described behavior + extensions. RESUME* not in static COMMANDS (unregistered for fuzzy) but functional via ai_commands — consistent with "slash commands" language in ADR.
  - ADR-130 (multi-account / TRADECOPY): accurate (TRADECOPY/TRADE_COPY/COPYTRADE; primary cycling; opt-in mirroring; uniform slots post-update).
  - Other ADRs (080, 083 AICACHE, 086 command dispatch split, 065 help/registry, 096 ingest): no major drift; mentions of palette/command/INGEST_RESEARCH/AICACHE match current registry + handlers.
  - No widespread stale "current state" claims in ADRs (they are dated records); "parity" and "stub" references mostly historical or intentional per prior sweeps. Console/command surface well covered without contradiction.
- Console command doc: KEYBOARD_SHORTCUTS now primary accurate reference (examples verified against registry + handlers). README/ROADMAP/ARCHITECTURE/DESIGN updated. In-app reference (workspace_reference_windows.rs) auto from COMMANDS (best source). Special AI console commands (ASK*, RESUME*) documented as such.
- No code changes; only doc accuracy fixes.

## 1. High-level Docs (ROADMAP.md, ARCHITECTURE.md, DESIGN_PHILOSOPHY.md, INDICATORS.md)

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

- [x] Multi-account brokers (ADR-130, 2026-07-02): Alpaca 4-slot account pool with round-robin bar-fetch fan-out + aggregate capacity scaling, Kraken extra trading identities, account-granular `Primary:` cycling (ADR-126 updated), TradeCopy window (one-shot position copy + live order mirroring), Sync Status drops disabled-TF rows from view + %, recent-fills WS/poll refresh + RFC3339 arrow-timestamp fix — documented in ADR-130 + API_KEYS.md + README ADR index.
- [x] Multi-account comb-over (2026-07-03): uniform Alpaca slots 1–4 (Key/Secret/Paper|Live only — per-slot Label/Trade/Data toggles removed, all slots sync + trade), credentials persist to keyring on field edit (fixes lost extra-account keys), `TRADECOPY` console command, order mirroring strictly opt-in per explicitly checked target (empty set = off, auto-off when emptied, never persisted) — ADR-130 §1/§3/§5 + Update section, API_KEYS.md, README, ROADMAP Phase 21.
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
