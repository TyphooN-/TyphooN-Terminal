# ADR-118: Test Module Decomposition Convention

**Status:** Accepted
**Date:** 2026-06-12
**Related:** ADR-086 (native app decomposition), ADR-108 (research modularization)

## Context

Two of the repo's largest files were not production code but **inline unit-test
modules** that had grown with their subject. `typhoon-engine/src/core/research/mod.rs`
carried a **~21,793-line `#[cfg(test)] mod tests`** (93% of the file, 1,030 tests),
and `typhoon-native/src/app/tests.rs` was a **3,463-line** standalone test module (179
tests). Several production files (`cache.rs`, `news.rs`, `sec_filing.rs`,
`alpaca.rs`, `sync_workset.rs`, `app_runtime_support.rs`, `kraken_ohlc_ws.rs`) also
carried 350–1,100-line inline `mod tests` blocks.

`#[cfg(test)]` keeps these out of normal `cargo check`/release builds, but they
still bloat the files for humans and rust-analyzer, and they are the reason
`research/mod.rs` read as a 23k-line monolith.

## Decision

Test code lives in its own file(s), via one of two mechanical, behavior-preserving
techniques.

### Technique 1 — inline `mod tests {}` → sibling dir-module `<name>/tests.rs`

When a production file `foo.rs` has a self-contained `#[cfg(test)] mod tests { ... }`,
replace it with `#[cfg(test)] mod tests;` and move the body to `foo/tests.rs`
(Rust 2018+ lets `foo.rs` and a `foo/` directory coexist). `use super::*` resolves
identically — the dir-module is still a child of `foo`. Run `rustfmt` on the moved
file to de-indent.

If the test module sits **mid-file** (production code follows it, e.g.
`kraken_ohlc_ws.rs` closed its `mod tests` at line 651 of 1018), extract only the
inner range, keep the trailing production code in place, and move the
`#[cfg(test)] mod tests;` declaration to the end of the file. **Do not assume the
test module runs to EOF** — verify by brace-matching from `mod tests {`.

### Technique 2 — large standalone `tests.rs` → `tests/` tree via `include!`

When a test file is large and its **fixtures are shared across sections**, split it
into per-area files under `tests/` and wire them from `tests/mod.rs` with
`include!("part.rs")` — **not** separate `mod` files.

The `include!` is deliberate and load-bearing. The fixtures (`synth_bars`,
`open_mem_conn*`, `mk_*` snapshot builders, `test_bar`, …) are defined in one
section and used by tests in others — e.g. `synth_bars` is defined at L1734 and
used through L2789, across a section boundary. `include!` textually concatenates
the slices into **one module scope**, so every fixture stays visible to every test;
the result is byte-identical compilation to the original single file. Splitting into
separate `mod` files would scope each fixture to one submodule and break callers in
the others.

> **Do not "clean up" an `include!` test tree into `mod` files** without first
> hoisting all shared fixtures into a `tests/common.rs` (`pub(super)`), then
> converting `include!` → `mod` and adding `use super::common::*` to each slice.

Conventions:
- An `include!`d slice must **not** carry its own `use super::*` — it lives once in
  `tests/mod.rs` before the includes.
- A dir-module `tests.rs` (Technique 1) **does** carry its own `use super::*`.

## What was done (2026-06)

- `research/mod.rs`: 21.8k-line inline `mod tests` → `research/tests.rs` → split into
  11 semantic files under `research/tests/` via `include!`.
- `typhoon-native/src/app/tests.rs`: 3.5k lines → 3 area files under `app/tests/` via
  `include!`.
- Inline `mod tests` → dir-modules: `typhoon-engine` `cache` / `news` / `sec_filing` /
  `alpaca`; `typhoon-native` `sync_workset` / `app_runtime_support` / `kraken_ohlc_ws`.
- Verified every step with `cargo check --tests` plus the full suites:
  **engine 1,638 passed, native 347 passed, 0 failed**. All pure moves.

## Consequences

- New tests go in a `tests` submodule **file**, never inline in a production file.
- Prefer Technique 1 (dir-module) for self-contained modules; use Technique 2
  (`include!`) only when fixtures are shared across the split.
- rust-analyzer and human navigation improve; `research/mod.rs` dropped from
  ~23.5k to **1,668** lines as a direct result.
- Test-binary compile time is unchanged (the crate is still one codegen unit); the
  win is readability, edit-locality, and rust-analyzer responsiveness.

## TODOs / deferred

- `typhoon-native/src/app/technical_analysis.rs` still has a ~220-line **mid-file**
  `#[cfg(test)] mod tests` (closes at L7542 of 7809). Extracting it barely dents the
  7.8k-line file, whose real weight is production rendering code — defer until that
  file gets its production split (ADR-086 next targets) and pull the tests then.
- Future large `tests/` slices can be further subdivided by family using the same
  `include!` rule.
