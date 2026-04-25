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
native/src/app/settings.rs         — Settings window
native/src/app/storage.rs          — Storage Manager + filtered bulk delete
native/src/app/sync_status.rs      — Sync Status (per-broker % healthy)
native/src/app/tool_windows.rs     — Indicator + analytical tool windows
native/src/app/strategy_windows.rs — Strategy / backtest / optimizer windows
```

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

`app.rs` is still the largest file in the crate (~155k lines), but the
peeled-off submodules now rebuild in isolation when they are the only
thing changed.

## Consequences

- **Edit-rebuild cycle is materially faster** for changes scoped to one
  submodule — Storage Manager edits no longer trigger an `app.rs`
  recompile, AI window edits no longer trigger Storage/Settings recompile.
- **No behavior change.** The split is structural; `git diff -M` shows
  near-perfect line moves for the six new files.
- **Window discovery is easier.** A new contributor looking for the AI
  Sessions browser or the Storage Manager finds it under the obvious
  submodule path instead of grepping a 158k-line file.
- **`app.rs` stays the integration point.** `TyphooNApp` remains in
  the parent file, all `BrokerCmd` / `BrokerMsg` handling lives there,
  the chart pane and command palette live there, and the central state
  graph (drawings, indicators, panes, sessions) is still defined in one
  place. The split is for renderer code, not for state.
- **Future renderers should land in submodules from day one.** The
  precedent is set: a new "X Window" renderer goes into
  `native/src/app/x_window.rs` (or a related bundle), not into `app.rs`.
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
