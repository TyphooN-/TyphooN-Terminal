# ADR-031: Dependency Version Alignment

**Status:** Implemented
**Date:** 2026-03-25

## Context

Multiple crates in the workspace had version splits causing duplicate compilation, slower builds, and potential security exposure from outdated transitive dependencies.

## Decision

Align all direct dependencies to their latest stable versions across the entire workspace. Eliminate version splits where the same crate was pinned at different versions in different workspace members.

## Changes (2026-03-25)

| Crate | Before | After | Notes |
|-------|--------|-------|-------|
| wgpu | 24 | **29** | Match eframe 0.34 internal version — eliminates naga duplicate |
| rfd | 0.15 | **0.17** | Native file dialog |
| pest / pest_derive | 2.7 | **2.8** | Parser (typhoon-transpiler) |
| rusqlite (cli) | 0.34 | **0.39** | Match engine version — was 5 versions behind |
| ratatui | 0.29 | **0.30** | TUI framework (cli) |
| crossterm | 0.28 | **0.29** | Terminal backend (cli) |
| rand (cli) | 0.9 | **0.10** | Match engine version |

## Follow-up alignment (2026-05-20)

The dependency audit was repeated with `cargo tree -d` and the workspace manifests were normalized further:

- Added `[workspace.dependencies]` for shared first-party crates and repeated third-party dependencies (`serde`, `serde_json`, `tokio`, `reqwest`, `chrono`, `tracing`, `tracing-subscriber`, `rusqlite`, `zstd`, credential crypto crates, and WebDriver plumbing). This keeps workspace members from drifting onto different direct versions.
- Ran `cargo update` to pick up compatible patch-level security/bugfix releases in `Cargo.lock`.
- Changed `reqwest` direct usage to `default-features = false` plus the explicit feature set actually used by the terminal (`json`, `query`, `cookies` where needed, and `rustls`). This avoids compiling reqwest's default TLS stack for HTTP clients that do not need it.
- Changed direct `thirtyfour` usage to `default-features = false` plus `reqwest`/`rustls`, omitting its component macro feature because TyphooN only uses the async WebDriver client for Darwinex Zero scraping. The old local `[patch.crates-io]` vendor override was removed after upgrading to upstream `thirtyfour` 0.37 because keeping a stale unused patch produces a Cargo warning and defeats the dependency-freshness policy.
- Changed native `eframe` from default features to explicit Linux native features (`default_fonts`, `wayland`, `x11`, `wgpu_no_default_features`). The direct `wgpu` dependency now enables only `std`, `parking_lot`, `vulkan`, and `wgsl`, avoiding web, DX12, Metal, GLES, and WebGPU backend feature compilation in the Linux native terminal.
- Revisited the `keyring` 4.x migration. The umbrella `keyring` 4.0.1 crate pulled in the optional SQLite/Turso backend and conflicted with `typhoon-native`'s global allocator, so TyphooN now uses the keyring 4.x split crates directly: `keyring-core` for `Entry`/`Error` and `dbus-secret-service-keyring-store` for Linux/FreeBSD Secret Service. This preserves the existing libsecret credential namespace without compiling the unused `keyring` CLI/sample wrapper, SQLite store, or Turso stack.

`native-tls` / `openssl` are still present because LAN sync currently builds a native-tls acceptor/connector for local WSS. Removing them safely requires a LAN-sync TLS implementation migration, not a manifest-only edit.

Explicit non-upgrades / unresolved upstream constraints after this pass:

- The old `keyring` 3.x crate is removed. Do not re-add the `keyring` 4.x umbrella crate unless the global-allocator conflict with its SQLite/Turso backend is resolved; prefer direct `keyring-core` + platform backend crates.
- `generic-array` 0.14.7 and `matchit` 0.8.4 are transitive exact-version constraints from upstream (`crypto-common` and `axum` respectively). `cargo update --verbose` plus explicit `cargo update -p … --precise …` attempts are the verification source for these residual non-upgrades; they cannot be forced from TyphooN manifests without patching upstream crates.

## Remaining Transitive Duplicates

These duplicates come from upstream crates and cannot be resolved without upstream updates:

- `calloop` 0.13/0.14 — Wayland compositor bindings (smithay)
- `getrandom` 0.2/0.3/0.4 — rand ecosystem transition
- `rustix` 0.38/1.1 — smithay/calloop transition
- `block-buffer` / `digest` / `crypto-common` / `hmac` / `sha2` 0.10/0.11 families — TyphooN's direct crypto deps intentionally stay on the latest stable line (`sha2` 0.11, `hmac` 0.13, `pbkdf2` 0.13); Secret Service and TLS transitive crates still use the 0.10 line.
- `thiserror` 1/2 — ecosystem-wide migration
- `hashbrown` 0.14/0.15/0.16/0.17 — graphics, SQL, CLI TUI, and HTTP stacks depend on different upstream lines.
- `zip` 7/8 — `calamine` still depends on zip 7 while engine directly uses zip 8

## Consequences

- **Pro:** Eliminates naga duplicate (largest version split — 2 full wgpu implementations)
- **Pro:** All workspace crates on latest stable versions
- **Pro:** Reduced compile time from fewer duplicate builds
- **Pro:** Security: no known CVEs in outdated pinned versions
- **Con:** Remaining transitive duplicates are documented above; direct TyphooN dependencies should move forward, not backward, even when an older transitive line remains.

## Follow-up alignment (2026-06-10)

Security-first refresh:

- Bumped direct pins in `[workspace.dependencies]` and member manifests to latest stable patch/minor releases discoverable via `cargo search` (tokio 1.52, eframe/egui/egui_* 0.34.3, ratatui 0.30.1, zeroize 1.8, base64 0.22.1, etc.).
- `rusqlite` deliberately left at 0.39 (0.40 pulls libsqlite3-sys requiring unstable `cfg_select` on the current rustc 2026-01-25 toolchain; concrete compatibility blocker).
- `aes-gcm` left at 0.10 (0.11 is still rc).
- `winit` left at 0.30.x (0.31 is beta).
- `cargo check --workspace` clean after updates.
- `cargo update --workspace` applied; no new version splits introduced in direct deps.
- Remaining duplicates unchanged from prior ADR (transitive only).

This keeps the policy: latest possible without breaking the build or introducing known-unstable crates.
