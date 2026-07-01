# Doc Drift Checklist (Living)

**Purpose**: Track mismatches between current implementation (code + runtime behavior) and documentation.  
**Style**: Prefer semantic names over old "feature parity" sequencing. Update on changes.  
**Maintenance**: Before major work, run searches for "stub|pending|not yet|parity|future|missing" in docs/ + cross-check key code areas. Mark items [x] when fixed and re-commit.  
**Last full sweep**: 2026-07 (broader sweep + L3 foundation + living checklist generation)  
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

## 2. Broker & Market Data (L1/L2/L3, Sync, Tiers)

- [x] L3 foundation (ws_v2_level3.rs, streamer with token/real/sim, CRC32 apply_with_checksum, KrakenL3State, received_at_ms, per-order, status, projection to same paths, Bookmap/depth integration, tests) — well covered in ADR-129 + ADR-109 Update sections.
- [x] L2 v2 book (CRC, exact tokens, 25 levels, v1 kept for compat) — documented; historical "pending" status lines in ADR-109 cleaned in 2026-07.
- [x] L1 (ticker/quotes with sizes, O(1) dispatch) — consistent.
- [x] Kraken Futures bars/sync — present in code (bar_fetch) and docs.
- [x] Alpaca as assist/fallback + trading (tier-autotuning, catalog-breadth) — matches ADRs 087/112/113.
- [x] M1/M5 rules (Kraken Spot/Equities valid low-TF targets; assist rows non-target) — referenced in floating-windows-perf-plan and memory, but not prominently in main ADRs/docs.
- [x] Sync schedulers / O(1) / coverage-first / AIMD / pending-work (ADRs 029, 087, 094, 098, 102, 107, 128) — language generally aligns with current bounded queues, backfill-complete markers, etc.
- [~] Many "pending work", "missing bars", "gap fill" references in sync ADRs — mostly accurate descriptions of ongoing mechanisms, not outdated claims.
- [x] v1 public_book.rs kept for legacy/compat — correctly noted in ADR-109.

## 3. Charts, UI & Visualization

- [x] Depth profile (live bins from 25 levels, "L3 depth" label + tint distinction, overlay) — in code (render.rs, app_runtime), now referenced in ARCHITECTURE + ADR-129.
- [x] Bookmap richer (per-order markers, scroll list, age coloring via received_at_ms + timestamp, interactions/click/copy, is_l3 detection) — covered in ADRs and recent updates.
- [x] MTF Grid parity (depth/L3 updates via chart_by_bare) — noted in ADR-129.
- [ ] Drawing tools count (89 in ROADMAP) and full list — verify against current implementation (some ADRs like 048 still use "parity with TradingView" in title).
- [ ] 46+ indicators claim in ROADMAP — directionally correct but exact count + GPU/CPU surface should be cross-checked periodically.
- [x] Session persistence, floating windows, right panel, watchlist — generally accurate in high-level docs.

## 4. Terminology & Historical Language (Avoid Old Sequencing)

- [ ] Titles and bodies containing "parity-with-mt5", "feature-parity-target", "xynth-feature-parity", "ux-parity" (ADRs 005, 048, 069, 092, 116) — historical ADRs; leave titles but soften body text where describing current state.
- [ ] "Parity" in active descriptive text (see section 1) — prioritize "computational equivalence", "parity surfaces", "equivalence" going forward.
- [~] "Stub" and redirect stubs in ADR/README and consolidated ADRs — intentional per 2026-06 consolidation. Acceptable.

## 5. ADRs with Potential Outdated Claims

- [x] ADR-109: Old Phase 1/2 status lines updated (2026-07) to reflect completion.
- [x] ADR-129: L3 plan, conclusion, prior-stubs clarification accurate post-foundation work.
- [ ] Other ADRs (e.g. 027 Bookmap-style, 017 MTF, 004 MTF indicators) — light or no coverage of modern depth profile / L3 / received_at_ms.
- [~] Large number of "pending", "future", "not yet" in sync/performance/research ADRs (78+ matches total) — mostly mechanism descriptions or P2 items. Review on per-ADR basis.
- [ ] ADR-116 (Finviz feature-parity-target) and similar — scope as historical target.

## 6. Other Areas

- [x] CLI/TUI references — correctly marked as removed/archived.
- [x] News vs market-data distinction (separate cache) — accurate.
- [ ] Kraken Futures trading vs market-data depth — lighter documentation (bars ok, full trading surface?).
- [x] Persistence (session.json, SQLite zstd, kv_cache) — matches.
- [ ] AI surfaces / research packet ingestion — well covered but check for drift on new surfaces.

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

---
*This is a living document. Treat it as the source of truth for doc maintenance priorities.*