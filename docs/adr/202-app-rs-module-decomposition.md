# ADR-202: native/app.rs Module Decomposition for Compile Speed

**Date:** 2026-04-23
**Status:** Accepted
**Related:** `native/src/app.rs`, `native/src/app/`, ADR-001 (native GPU architecture)

## Context

`native/src/app.rs` had grown past 158k lines as the home of `TyphooNApp`
plus every floating-window renderer, every command handler, every keyboard
binding, and every popup body in the native UI. Even small UI tweaks
recompiled the whole file because Rust's compilation unit is the crate, not
the function — but the practical bottleneck was each `cargo build` cycle
running through the full `app.rs` typecheck plus codegen. Iteration feel
was 30s+ for a one-line change.

The file's growth pattern was clear from blame: window renderers were
top-of-file additions, and there was no internal seam to peel off without a
deliberate split. Splitting along feature boundaries (Storage Manager,
Sync Status, Settings, AI windows, indicator-tool windows, strategy
windows) was the obvious move because each renderer was already
self-contained — it took an `&mut TyphooNApp` and an `egui::Ui`, and
otherwise touched only its own state.

## Decision

Decompose `native/src/app.rs` into a `native/src/app/` submodule tree,
moving the largest self-contained renderers and leaving `TyphooNApp` plus
the chart / palette / command-dispatch core in the parent file.

**Final layout:**

```
native/src/app.rs                  — TyphooNApp, chart, palette, dispatch
native/src/app/ai.rs               — AI Chat, Claude Code, Gemini CLI,
                                     Codex CLI, AI Sessions, AI Response
                                     Cache (six related windows)
native/src/app/alpaca_sync.rs      — broker sync capacities, TF filters,
                                     no-data/backfill-complete marks
native/src/app/auto_compact.rs     — zstd-22 idle auto-compact gate
                                     and schedule helpers
native/src/app/bar_sync.rs         — bar-sync health aggregates for
                                     Sync Status and Storage Manager
native/src/app/broker_fetch.rs     — async broker bar-fetch workers and
                                     response normalization
native/src/app/settings.rs         — Settings window
native/src/app/storage.rs          — Storage Manager + filtered bulk delete
native/src/app/sync_status.rs      — Sync Status (per-broker % healthy)
native/src/app/tool_windows.rs     — Indicator + analytical tool windows
native/src/app/strategy_windows.rs — Strategy / backtest / optimizer windows
```

The original 2026-04-23 split moved the six renderer bundles. Subsequent
compile-speed and storage/sync passes added `alpaca_sync.rs`,
`auto_compact.rs`, and `bar_sync.rs` so sync policy and scheduler code no
longer live as anonymous helper islands inside `app.rs`.

The 2026-05-20 sync/compile pass added `sync_config.rs` for broker sync
budgets and tastytrade timeframe-window helpers, then moved the async broker
bar-fetch worker functions into `broker_fetch.rs`. The parent `app.rs` still
owns state and message routing, but HTTP/DXLink/Kraken response parsing and
task completion helpers now compile as their own app submodule.

Each submodule is a sibling of `app.rs`, declared as `mod` from the parent.
Window functions take `&mut self` on `TyphooNApp` so state mutation works
exactly as before — no trait abstraction, no event bus, no message passing.
The split is mechanical: cut the function bodies out, paste them into the
new file, add `use` lines for the types they reference.

The two-step split was deliberate:

- **8aa81937 (2026-04-23 07:36)** — first peel: Storage, Sync Status,
  Settings, AI windows. The six AI windows (AI Chat, Claude Code, Gemini
  CLI, Codex CLI, AI Sessions, AI Response Cache) are tightly related and
  are now bundled in `ai.rs` rather than one module each, because they
  share the same provider plumbing from ADR-157.
- **1c667fb0 (2026-04-23 09:03)** — second peel: tool windows and
  strategy windows.

The original ~158k-line `app.rs` has since been reduced to roughly 33k
lines. The current largest seam is `native/src/app/floating_windows.rs`
(~63k lines), followed by `command_palette.rs` (~18k lines). Peeled-off
submodules rebuild in isolation when they are the only thing changed.

## Consequences

- **Edit-rebuild cycle is materially faster** for changes scoped to one
  submodule — Storage Manager edits no longer trigger an `app.rs`
  recompile, AI window edits no longer trigger Storage/Settings recompile.
- **No behavior change.** The split is structural; `git diff -M` shows
  near-perfect line moves for the six new files.
- **Window discovery is easier.** A new contributor looking for the AI
  Sessions browser or the Storage Manager finds it under the obvious
  submodule path instead of grepping a monolithic parent file.
- **`app.rs` stays the integration point.** `TyphooNApp` remains in
  the parent file, all `BrokerCmd` / `BrokerMsg` handling lives there,
  the chart pane and command palette live there, and the central state
  graph (drawings, indicators, panes, sessions) is still defined in one
  place. The split is for renderer code, not for state.
- **Future renderers should land in submodules from day one.** The
  precedent is set: a new "X Window" renderer goes into
  `native/src/app/x_window.rs` (or a related bundle), not into `app.rs`.
- **Future broker/sync policy should land in sync modules from day one.**
  Scheduler budget constants and helper functions belong in
  `app/sync_config.rs`, selector logic in `app/alpaca_sync.rs` or a broker-
  specific sync module, and queue/refill orchestration in
  `app/market_data_sync.rs`; do not add new sync islands to `app.rs`.
- **The remaining largest seams are mechanical but high-touch:**
  `floating_windows.rs` should be split by window family, `command_palette.rs`
  by command namespace, and any new broker fetch worker should land in
  `app/broker_fetch.rs` instead of being added back to `app.rs`.
- **The default launcher path must stay incremental-friendly.** `./launch.sh`
  runs the thin-LTO `release` profile for normal use; full-LTO
  `release-max` remains available as `./launch.sh max` only for explicit final
  artifact builds.
- **No public API change.** All split functions remain `impl TyphooNApp`
  methods; nothing in the engine, web, or web-server crates needs to
  notice.

## Verification

- `cargo build --workspace` clean before and after each split commit.
- `cargo test --workspace --lib` test counts unchanged across the split.
- Spot-check: clean rebuild of `native` after touching only
  `app/storage.rs` should not rebuild any non-affected `app/*.rs`
  module's object file (verified by `cargo build -v` output showing
  one `rustc` invocation per touched module).
