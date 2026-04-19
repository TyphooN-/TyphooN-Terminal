# ADR-188: Chart-Drawing Parity Deferred — Research-Packet-First

**Status:** Accepted
**Date:** 2026-04-19
**Related:** ADR-004 (chart engine), ADR-011 (indicator system), ADR-032 (Ehlers DSP), ADR-108..ADR-187 (TA-Lib + Godel Parity Rounds 1..75), ADR-067 (feature completeness audit)

## Context

Rounds 1–75 of what was originally named "Godel Parity" have added
~375 TA-Lib primitives (indicators + candlestick pattern detectors)
plus a number of genuinely Godel-Terminal-documented research
features (options chain, expirations calendar, earnings whispers,
institutional ownership, etc.). The work lives entirely in the
**research layer**:

- `engine/src/core/research.rs` — snapshot structs, compute fns,
  SQLite tables, upsert/get wrappers.
- `engine/src/core/lan_sync.rs` — `SYNCABLE_TABLES` + schema mirror.
- `native/src/app.rs` — `BrokerCmd` / `BrokerMsg` round-trip, App
  fields/defaults, tokio broker handlers, palette alias blocks,
  packet emitters (markdown sections consumed by the AI research
  packet), and egui **popup windows** (Use-Chart / Load-Cached /
  Compute controls + Grid summary).

None of this work touches the **chart drawing layer** — no overlays,
no sub-panels, no per-bar detection replay rendered on top of the
candle stream in `native/src/chart/`. A user viewing AAPL's chart
sees the ordinary price/volume candles; to inspect the latest
detected CDLHAMMER / CDLTRISTAR / harami cross they open the
palette popup (e.g. `CDLTRISTAR`) or read the section in the AI
research packet.

Round 75's classification audit surfaced the scope question
explicitly: CDL\* primitives are **TA-Lib**, not documented Godel
Terminal features. The research packet *is* a Godel-adjacent
differentiator — AI agents consuming it benefit from these signals
regardless of whether they are rendered on a chart. The user's
direction: **continue TA-Lib parity rounds, scoped to research-
packet support; defer chart drawing parity to a future, separately-
tracked ADR series.**

## Decision

1. **Program rename** (already applied from R75 forward):
   "Godel Parity" → **"TA-Lib + Godel Parity"**. Each round ADR
   carries a per-feature classification table marking whether the
   addition is Godel-Terminal-documented, a TA-Lib primitive, or
   both. ADR-108..ADR-186 will be backfilled by a single audit pass
   (tracked separately).
2. **Research packet is the first-class target.** All ongoing
   TA-Lib + Godel parity rounds ship features through the
   research-layer pipeline (snapshot struct → SQLite → LAN-sync
   whitelist → BrokerCmd/Msg → packet emitter → egui popup window).
   This is the minimum viable path for an AI-agent-visible feature.
3. **Chart-drawing parity is deferred**, explicitly, until a future
   ADR reopens the track. Rationale:
   - Chart-overlay work has a materially larger per-pattern cost:
     it touches `native/src/chart/` draw layers, per-bar detection
     replay (not just latest-match snapshot), pan/zoom performance
     budgeting, palette visibility toggles, style/colour mapping per
     pattern-family, and interaction with existing chart indicators
     (ADR-004/011/032). A realistic round cadence for chart overlays
     is **1–2 patterns per round**, not 5.
   - The AI-agent consumer (the packet reader) does not need chart
     marks — it reads the markdown section directly. The highest-
     leverage use of round budget is closing out the remaining
     TA-Lib catalogue and explicitly-Godel-documented features
     *before* spending rounds on UX polish.
   - No new free-tier data sources or engine capabilities are
     unlocked by chart marks — they are purely a visual rendering of
     signals that already exist in the research layer.
4. **Future research rounds make no chart-overlay claims.** Each
   classification table will show the "Chart overlay" column as
   `No (deferred — ADR-188)` for any feature that lacks chart
   wiring. If a future round ships chart wiring for a specific
   feature (as part of the chart-parity reopening), that column
   flips to `Yes` for that row only.

## What is in scope for a future chart-parity track

Whenever the chart-parity track reopens (no ETA in this ADR), the
first candidates will be the highest-signal-density research-layer
features that already have:
- latest-match bar timestamps (easy mark placement),
- a well-defined TA-Lib `+100` / `-100` / `0` tri-state (easy
  bull/bear colour encoding),
- cross-timeframe relevance (chart users typically want to see them
  regardless of indicator selection).

Likely first batch (for reference; not binding):
- CDLHAMMER / CDLSHOOTINGSTAR (R72) — single-bar reversal triangles
  below/above the detected bar, bullish/bearish colour.
- CDLENGULFING / CDLHARAMI (R72) — 2-bar brackets spanning the pair.
- CDLMORNINGSTAR / CDLEVENINGSTAR (R73) — 3-bar brackets.

Each such pattern would need: draw-layer hook, per-bar detection
replay (not just the cached latest-match), a palette toggle, a
style token, and a performance budget check against the chart
frame budget. None of that is in R72..R75.

## What stays in scope for ongoing rounds

- Research-packet scope for every TA-Lib primitive we add.
- Godel-documented feature gaps (options flow, institutional
  ownership deltas, short-interest trajectory, sector heatmap
  snapshots, dividend / IPO / earnings / economic calendars,
  analyst ratings, insider transactions, etc.) — these go into the
  research packet via the same pipeline.
- Chart-layer work for features that already ship as chart
  indicators remains in-scope under the existing indicator ADRs
  (ADR-004/011/032) — this deferral applies only to the
  TA-Lib + Godel Parity round cadence.

## Consequences

- **User-visible gap:** no candlestick pattern marks drawn on the
  chart. Discoverability of cached CDL\* detections relies on the
  palette (e.g. typing `CDLTRISTAR`) or reading the research packet.
- **AI-agent consumers are unaffected** — the research packet carries
  every detection as a markdown line regardless of chart-layer
  status.
- **Round cadence stays fast** — 5 primitives per round is
  sustainable at research-only scope; it would not be sustainable
  at chart-overlay scope.
- **Classification clarity** — every round ADR from R75 onward
  declares whether each feature is Godel-documented, TA-Lib-
  primitive, or both. ADR-108..ADR-186 get backfilled.
- **Schema debt is avoided** — no chart-overlay tables, no per-bar
  detection replay caches, no palette toggle state. If/when the
  chart-parity track reopens, those can be introduced cleanly
  without retroactive changes.

## Verification

- No code changes in this ADR — it is a scope-setting document.
- Implementation check: grep `native/src/chart/` for any
  `CDL` / candlestick-pattern draw references — should return
  nothing, confirming no accidental chart-layer drift in R72..R75.
- ADR-187 (R75) is the first round ADR with a classification table;
  subsequent rounds follow the same template.

## Revisiting this decision

The chart-parity track may be reopened with a new ADR when any of:
- TA-Lib CDL\* catalogue is materially complete (≥ 80% of the
  ~61 TA-Lib CDL\* primitives landed — currently ~20 / ~61 after R75).
- Explicit user UX feedback that chart marks are blocking adoption.
- A parallel indicator-system refactor (e.g. an ADR-004 successor)
  lands that makes per-pattern chart marks cheap to wire.

Until then, every round ADR continues to declare chart overlay as
`No (deferred — ADR-188)` for new research-layer additions.
