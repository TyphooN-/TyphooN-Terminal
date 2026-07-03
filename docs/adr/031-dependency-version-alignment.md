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

## Follow-up alignment (2026-07-02)

Latest-stable refresh; every blocker documented in the 2026-06-10 pass was
re-tested and cleared:

- **Toolchain**: `rustup update nightly` — rustc 1.95.0-nightly (2026-01-25)
  → 1.98.0-nightly (2026-07-01). The five-month-stale nightly was itself the
  `rusqlite 0.40` blocker (`libsqlite3-sys 0.38` uses the since-stabilized
  `cfg_select!` in its build script). The workspace intentionally has no
  `rust-toolchain.toml`; the default nightly channel is expected to be kept
  current as part of this policy.
- **`rusqlite` 0.39 → 0.40.1** — bundled SQLite 3.51.3 → **3.53.2** for the
  research cache / bar store / KV. No API breakage.
- **`aes-gcm` 0.10 → 0.11** — no longer rc; aligns the AEAD path with the
  RustCrypto hybrid-array line already in tree (`sha2` 0.11 / `hmac` 0.13).
  Two `Nonce::from_slice` call sites in `typhoon-engine/src/core/cache.rs`
  moved to `From<[u8; 12]>` / `TryFrom<&[u8]>`. `generic-array 0.14` now
  survives only under `dbus-secret-service` (Secret Service keyring), no
  longer under TyphooN's own crypto path.
- **egui stack 0.34.3 → 0.35.0** (`eframe`/`egui`/`egui_extras` 0.35,
  `egui_plot` 0.36, `egui_commonmark` 0.24) — eframe 0.35 removed the
  deprecated `App::update`; the native frame body moved to `App::ui` with
  chrome panels rendering through the root `Ui`. Deliberately kept as one
  body (no `logic()`/`ui()` split): eframe 0.34 already gated `update()`
  behind `is_visible`, so a hidden window pausing the pump is long-standing
  shipped behavior, and one body preserves it exactly.
- **`wasm-encoder` 0.250 → 0.252** — transpiler codegen; no source edits.
- **Durable rule (new): the direct `wgpu` major must follow `egui-wgpu`'s,
  not crates.io latest.** wgpu 30 is published, but egui-wgpu 0.35 pins
  wgpu 29; bumping the direct dep ahead of eframe creates a dual-major wgpu
  tree whose types cannot unify with eframe's render state. wgpu 30 arrives
  with the egui release that adopts it.

Residual constraints after this pass: `generic-array 0.14.7` and
`matchit 0.8.4` remain transitive exact-version holds (unchanged owners);
`quick-xml 0.39.4` remains behind its 0.41 advisory fix pending a
wayland-scanner release — acceptance documented in `.cargo/audit.toml` and
ADR-088. `winit` stays 0.30.x with eframe 0.35.

`cargo audit` clean; full workspace suite 2403 passed / 0 failed.

## Lean sweep (2026-07-03) — minimal features, unused deps, framework removal

Full pass over every direct dependency's feature flags plus a
`cargo-udeps` scan. Lockfile: **609 → 580 crates**. Suite green (2404),
`cargo audit` clean.

Removed outright:

- **`axum` (and with it `axum-core`, `matchit`, `tower`, `tower-layer`,
  `serde_path_to_error`, …)** — the framework served exactly one GET route,
  the Prometheus `/metrics` text endpoint. Replaced by a ~60-line hand-rolled
  HTTP/1.1 responder on `tokio::net::TcpListener` (`typhoon-native/src/metrics.rs`,
  covered by an end-to-end socket test). This also retires the
  `matchit =0.8.4` exact-pin from the "behind latest" list — the crate is no
  longer in the tree. **Security tightening in the same change: the metrics
  server now binds `127.0.0.1` by default** (the payload names account equity
  and open-position counts); `TYPHOON_METRICS_BIND=0.0.0.0` opts back in to
  LAN scraping.
- **Unused dependencies (cargo-udeps verified by grep):** `typhoon-engine` no
  longer declares `tracing-subscriber` (only `tracing` is used; subscriber
  init lives in the binary) or `typhoon-transpiler` — dropping that edge also
  removes engine→transpiler from the build graph, so the two build in
  parallel. `typhoon-transpiler` drops unused `serde_json` and
  `pretty_assertions`.

Feature minimization (each verified against actual usage):

- **`egui_extras`**: `all_loaders` → `["image", "webp", "http"]`. Drops the
  svg loader (`resvg`/`usvg`/`tiny-skia`/`kurbo` + a duplicate `png 0.17`)
  and the gif decoder; finance-news imagery is raster over URLs. `http`
  (ehttp/ureq/ring) stays — it is what fetches article images.
- **`prometheus`**: `default-features = false` — text exposition only, drops
  `protobuf`/`protobuf-support`.
- **`zstd`**: `default-features = false, features = ["arrays"]` — cache blobs
  are modern zstd frames; the legacy v0.5–0.7 format support and the
  dictionary builder never compile.
- **`keyring-core`**: the `sample` (in-memory test store) feature moved to
  dev-dependencies; the shipped binary carries only Entry/Error + the Secret
  Service backend.

Kept deliberately, with reasons recorded at the declaration site:

- `serde_json` `preserve_order` — `indexmap` remains in-tree via wgpu/naga
  regardless, and dropping it would reorder every `Value` iteration
  (research-packet/session display order) for zero tree savings. `raw_value`
  is load-bearing (exact wire tokens for the Kraken book checksum).
- `reqwest` `cookies` — the Yahoo crumb/consent flow requires a cookie jar.
- `rfd` defaults (`xdg-portal` + `wayland`) — correct minimal set for this
  desktop; no gtk in tree.

Confirmed immovable (upstream-final):

- `wgpu` 30 — blocked by the egui-wgpu pairing rule above.
- `generic-array 0.14.7` — `crypto-common 0.1` pins `=0.14.7`; reached only
  through `dbus-secret-service 4.1.0`, which is the **latest** release and
  still builds on the old RustCrypto line. The whole old-line duplicate
  family (`aes` 0.8, `cipher` 0.4, `digest` 0.10, `hmac` 0.12, …) unifies
  only when that crate migrates.
