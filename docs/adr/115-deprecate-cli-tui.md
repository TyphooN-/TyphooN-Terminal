# ADR-115: Deprecate CLI/TUI and Archive It to Branch

## Status

Accepted — 2026-06

## Context

TyphooN Terminal is currently optimizing for the native GUI as the primary product surface. The standalone `typhoon-cli` Ratatui interface duplicated active product surface area, pulled terminal-only dependencies into workspace checks, and distracted compile-time/refactor work from the GUI path.

The CLI/TUI may still be useful later for SSH/headless workflows, cache import/export, and MCP-style research packets, but it is not the active focus now.

## Decision

Remove the `cli` crate from the active `master` workspace and delete the active `cli/` source tree from `master`. Preserve the implementation on the pushed archive branch:

- `deprecated/cli-tui`

The active workspace remains focused on:

- `typhoon-engine`
- `typhoon-broker-runtime`
- `typhoon-chart-ui`
- `typhoon-native`
- `typhoon-research-ui`
- `typhoon-transpiler`

## Consequences

- `cargo check --workspace` no longer builds the CLI/TUI or its Ratatui/Crossterm/Clap dependency surface.
- GUI compile-time and modularization work has less unrelated workspace churn.
- README, architecture, and roadmap docs no longer advertise the CLI/TUI as an active surface.
- Historical CLI/TUI code remains recoverable from `deprecated/cli-tui`.

## Reintroduction Criteria

If the CLI/TUI comes back, it should return as a deliberate branch/PR with:

1. A clearly scoped owner/use case: SSH/headless ops, MCP packet server, or cache tooling.
2. A dependency budget that does not slow GUI iteration.
3. Tests/checks that run independently from the GUI-critical path.
4. Updated docs that mark it active again.
