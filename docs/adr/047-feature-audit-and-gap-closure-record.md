# ADR-047: Feature Audit and Gap-Closure Record

**Status:** Current as historical closure record
**Date:** 2026-04-02
**Updated:** 2026-05-27
**Supersedes:** Old ADR 067, old ADR 069, old ADR 087, old ADR 088

## Context

The old ADR set had separate feature-audit snapshots, follow-up closure passes, and gap-list records. They were useful during the sprint, but as permanent ADRs they created stale references and made onboarding harder.

This ADR is the compact historical record: what kind of gaps were found, how they were closed, and how future audits should be handled.

## Decision

- Feature audits are status records, not standalone architecture unless they change a system boundary.
- Closure passes should land code/docs fixes, then collapse into this record instead of becoming permanent top-level ADR spam.
- Data/provider/entitlement-gated items must be labeled as roadmap-gated, not left as ambiguous unfinished work.
- Future audit output should live in `docs/ROADMAP.md`, `docs/RESEARCH_PACKET.md`, or the affected feature ADR unless it establishes a new architectural rule.

## Compacted audit inventory

| Old ADR | Compacted topic | Old status | Old date |
| --- | --- | --- | --- |
| 067 | Feature Completeness Audit | Complete | 2026-04-02 |
| 069 | Feature Status & Roadmap (2026-04-05) | Historical snapshot, superseded by later implementation ADRs | 2026-04-05 |
| 087 | Close Out Remaining ADR Follow-ups (Help auto-gen, Scope Indicator, Session Persistence, EV/Fund Scope, ICS Export, Alert Attention) | Implemented | 2026-04-09 |
| 088 | Close old ADR 069 Feature Gap List | Implemented | 2026-04-09 |

## Durable outcomes

- Broker command/message routing, drawing editability, option-chain display, notifications, MT5 sync behavior, watchlist editing, and research/fundamental windows were brought into production-grade shape over the closure passes.
- Test-count snapshots in the old records were historical and intentionally not preserved as active claims here.
- Remaining roadmap-scale items should be reopened only with a concrete provider, UI, or architecture target.
