# Doc Drift Checklist (Living)

**Purpose**: Track mismatches between current implementation (code + runtime behavior) and documentation.  
**Style**: Prefer semantic names over old "feature parity" sequencing. Update on changes.  
**Maintenance**: Before major work, run searches for "stub|pending|not yet|parity|future|missing" in docs/ + cross-check key code areas. Mark items [x] when fixed and re-commit.  
**Last full sweep**: 2026-07-02 pm (dependency currency pass: ADR-031/088 follow-up sections — nightly 2026-07-01, rusqlite 0.40/SQLite 3.53.2, aes-gcm 0.11, egui 0.35 `App::ui` migration, wgpu-follows-egui-wgpu rule, `.cargo/audit.toml` acceptance practice; ADR-108 research-ui render segmentation; ARCHITECTURE research-ui line). Prior same-day: broker-modular capability model + typed provenance + WS self-heal doc sync; ARCHITECTURE crate-structure + ADR-count corrections; README ADR index + RESEARCH_PACKET broker-scope/key drift.  
**Status legend**:
- [ ] Open drift (code ahead or doc outdated)
- [x] Fixed / in sync
- [~] Acceptable historical (intentional record, git-history pointer, or ADR title)

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

## 2. Broker & Market Data (L1/L2/L3, Sync, Tiers)

- [x] Multi-account brokers (ADR-130, 2026-07-02): Alpaca 4-slot account pool with round-robin bar-fetch fan-out + aggregate capacity scaling, Kraken extra trading identities, account-granular `Primary:` cycling (ADR-126 updated), TradeCopy window (one-shot position copy + live order mirroring), Sync Status drops disabled-TF rows from view + %, recent-fills WS/poll refresh + RFC3339 arrow-timestamp fix — documented in ADR-130 + API_KEYS.md + README ADR index.
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
- [x] Session persistence, floating windows, right panel, watchlist — generally accurate in high-level docs.

## 4. Terminology & Historical Language (Avoid Old Sequencing)

- [x] Titles and bodies containing "parity-with-mt5" etc. (ADRs 005, 048, 069, 092, 116) — titles left as historical references; bodies softened (e.g. "visual equivalence", "TradingView-style", "Client equivalence").
- [x] "Parity" in active descriptive text — softened across high-level and ADR bodies to "equivalence", "style", "computational match" (prioritized going forward).
- [~] "Stub" and redirect stubs in ADR/README and consolidated ADRs — intentional per 2026-06 consolidation. Acceptable.

## 5. ADRs with Potential Outdated Claims

- [x] ADR-109: Old Phase 1/2 status lines updated (2026-07) to reflect completion.
- [x] ADR-129: L3 plan, conclusion, and prior-stub wording now reflect the gated real/sim implementation.
- [x] Other ADRs (e.g. 027 Bookmap-style, 017 MTF, 004 MTF indicators) — updated with modern depth profile / L3 / received_at_ms / MTF propagation.
- [~] Large number of "pending", "future", "not yet" in sync/performance/research ADRs (78+ matches total) — mostly mechanism descriptions or P2 items. Review on per-ADR basis.
- [x] ADR-116 (Finviz feature-parity-target) and similar — scoped as historical reference audit / gap-closure plan.

## 6. Other Areas

- [x] CLI/TUI references — correctly marked as removed/archived.
- [x] News vs market-data distinction (separate cache) — accurate.
- [x] Kraken Futures trading vs market-data depth — clarified in API_KEYS.md (primarily data for Futures; full private trading for crypto/xStocks).
- [x] Persistence (session.json, SQLite zstd, kv_cache) — matches.
- [x] AI surfaces / research packet ingestion — well covered but check for drift on new surfaces.
- Code work on gated items (sim/demo side): L3 sim/demo remains available for entitlement-free testing; Bookmap L3 selection is now real per-window state with row/header detail/heatmap marker highlighting, not a local stub.

## How to Use This Checklist
1. Run: `grep -rE 'stub|pending|not yet|parity|future work|in progress' docs/ --include="*.md" | head -30`
2. Cross-check code: `grep -r "depth_profile\|L3\|Bookmap\|received_at_ms\|chart_by_bare" --include="*.rs" typhoon-native/src/app/ typhoon-chart-ui/ | head -10`
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

---
*This is a living document. Treat it as the source of truth for doc maintenance priorities.*
