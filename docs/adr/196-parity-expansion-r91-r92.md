# ADR-196: Parity Expansion R91-R92 — Deferred Sector Liquidity and Benchmark-Link Ranks

**Status:** Accepted  
**Date:** 2026-04-21  
**Extends:** ADR-195  
**Related:** `engine/src/core/research.rs`, `engine/src/core/lan_sync.rs`, `native/src/app.rs`, ADR-125 (LIQRANK), ADR-128 (future-work list), ADR-195 (CORRSTK / MOMRANK_MULTI), ADR-188 (chart-drawing parity deferred)

## Context

After ADR-195 landed the two long-deferred cache dependencies
`MOMRANK_MULTI` and `CORRSTK`, the next clean parity gaps were the
follow-through ranks that earlier ADRs had postponed:

- `TLRANK` — 30-day liquidity rank vs sector peers
- `CORRRANK` — benchmark-linkage rank vs sector peers

Both had been deferred for practical reasons rather than conceptual
ones:

- `TLRANK` originally required a per-peer historical-price scan that
  did not fit the earlier “no fresh peer HP scans” envelope.
- `CORRRANK` was blocked on there being no reusable benchmark-linkage
  surface to rank. `CORRSTK` now provides that basis directly.

At this point both are reasonable:

- `TLRANK` can compute one narrow `30d` ADV$ scan on demand for the
  subject sector only, without inventing a whole parallel cache family.
- `CORRRANK` can reuse cached `CORRSTK` snapshots and rank peers on one
  benchmark basis (`SPY` or the mapped sector ETF) without new market
  fetches.

## Scope

### Round 91

- `TLRANK`

### Round 92

- `CORRRANK`

Both are research-packet and popup surfaces only. Chart overlays remain
deferred under ADR-188.

## Decision

Ship both deferred peer-rank surfaces as one bundled pass.

Implementation details:

- `TLRANK`
  computes trailing `30`-session average dollar volume from cached daily
  bars for the subject and same-sector peers, assigns the existing
  absolute liquidity tier thresholds, and percentile-ranks the subject
  within the sector cohort.
- `CORRRANK`
  reuses cached `CORRSTK` snapshots, chooses one benchmark basis from
  the subject row (sector ETF when that is the valid dominant benchmark,
  otherwise the market benchmark), and percentile-ranks `|corr_252d|`
  versus same-sector peers on that same basis.

The additive schema layer is:

- `v90`: `research_tlrank`, `research_corrrank`

For every feature in this ADR:

- Research packet: yes
- egui popup: yes
- Palette aliases: yes
- LAN sync: yes
- Chart overlay: no, deferred by ADR-188

## Consequences

### Positive

- `TLRANK` closes the gap between the broad `LIQRANK` cache-based view
  and the practical “what is liquid right now?” question.
- `CORRRANK` turns `CORRSTK` from a standalone diagnostic into a sector
  comparison surface, which is the actual parity endpoint the older
  future-work lists were pointing at.
- No new external APIs or fetch flows are required for either surface.

### Negative / risks

- `TLRANK` still performs a sector-local HP scan on demand, so very wide
  sectors with many cached peers cost more than cache-only rank surfaces.
- `CORRRANK` depends on `CORRSTK` already being cached for peers; sparse
  benchmark-linkage coverage still yields `NO_DATA`.
- The additive schema chain grows to `v90`.

### Neutral

- This continues the bundled ADR pattern rather than re-fragmenting the
  parity trail into tiny one-feature docs.
- `CORRRANK` explicitly ranks linkage, not desirability; the UI and
  packet state that higher percentile means tighter benchmark sync.

## Verification

- `cargo test --manifest-path engine/Cargo.toml tlrank -- --nocapture`
  verifies `TLRANK` roundtrip and compute coverage.
- `cargo test --manifest-path engine/Cargo.toml corrrank -- --nocapture`
  verifies `CORRRANK` roundtrip and compute coverage.
- `cargo test --manifest-path engine/Cargo.toml corrstk -- --nocapture`
  re-checks the upstream benchmark-linkage surface that `CORRRANK`
  depends on.
- `cargo check --manifest-path native/Cargo.toml`
  verifies the broker wiring, cache persistence, packet output, palette
  aliases, and popup windows.
