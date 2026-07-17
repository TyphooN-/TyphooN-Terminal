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

## Lean sweep, round 2 (2026-07-03 pm)

Re-verified the whole surface a day after the first sweep, then took the
feature trims deliberately deferred from it. Version state: `cargo update
--dry-run --verbose` shows **zero** compatible updates pending — the tree is
at ceiling — and the three upstream blockers were re-checked against
crates.io and are unchanged (egui-wgpu 0.35.0 is still the latest and pins
wgpu 29; dbus-secret-service 4.1.0 is still the latest and still on the old
RustCrypto line; wayland-scanner 0.31.10 still pins quick-xml ^0.39).
`cargo audit` clean. Lockfile 580 → **579**.

- **`futures-util` declared minimal per crate** (was bare `"0.3"` with
  defaults in three crates): engine `["std", "sink"]` (StreamExt + SinkExt
  for the WS lanes — `sink` was previously enabled only transitively by
  tokio-tungstenite, i.e. one upstream feature change from a build break);
  broker-runtime `["alloc"]` (join_all only); **typhoon-native's dependency
  removed outright** — its only "futures" matches were the
  `core::kraken_futures` module. Drops the `futures-macro` proc-macro from
  the tree entirely.
- **`aes-gcm`**: `default-features = false, features = ["aes", "alloc"]` —
  backup-encryption nonces come from `rand::random`, so the AEAD's
  `getrandom` default was dead weight.
- **`zeroize`**: dropped the `derive` feature — the engine uses
  `Zeroizing<String>` and the `Zeroize` trait, no derives. (zeroize_derive
  remains in the tree via another dependent, but is out of the engine's
  build graph.)
- **`tokio`**: native drops `fs` (no `tokio::fs` use; broker-runtime owns
  async file I/O and keeps it).
- Checked and already minimal: `egui_plot` (default = []), `rand` 0.10
  defaults (exactly the `rand::random` requirements), rusqlite
  (`bundled` only), scraper/image/wgpu/eframe/reqwest per-crate sets from
  the first sweep.

## Follow-up alignment (2026-07-06)

Security-first refresh with the same "latest stable, minimum direct feature
surface, no workspace drift" rule:

- Ran `cargo upgrade`, `cargo update` against every lagging compatible package
  reported by Cargo, `cargo outdated --workspace --root-deps-only`,
  `cargo tree -d --workspace`, `cargo audit`, and a manifest drift scan over
  every workspace `Cargo.toml`.
- Direct requirement bumps: `zeroize` 1.8 → 1.9 in `[workspace.dependencies]`,
  `serial_test` 3.2.0 → 3.5.0 for engine tests. No direct dependency now has
  multiple explicit version requirements across workspace manifests.
- Lockfile-compatible refreshes: `cc` 1.2.66, `dbus` 0.9.12, `hashlink`
  0.12.1, `jobserver` 0.1.35, `num-bigint` 0.4.8, `pest`/`pest_derive`/
  `pest_generator`/`pest_meta` 2.8.7, `pxfm` 0.1.30, `quinn-proto` 0.11.16,
  `quinn-udp` 0.5.15.
- Removed the `num-bigint` 0.4.7 yanked-warning surface by moving to 0.4.8.
  `cargo audit` is clean with only the documented quick-xml advisory
  acceptances in `.cargo/audit.toml`.
- Feature surface remained intentionally minimal. No upstream default feature
  expansions were reintroduced; the lockfile shrink from the `quinn-udp`
  refresh removed the old `windows-sys 0.60` target package set.
- Confirmed blockers unchanged: `wgpu` 30 is still blocked by `egui-wgpu`
  0.35.0's wgpu 29 pairing, and `generic-array` 0.14.7 remains pinned through
  upstream `dbus-secret-service` / old RustCrypto transitive dependencies.

Direct `wgpu` follow-up after the refresh:

- GPU-compute code imports wgpu types through `eframe::wgpu`, because TyphooN
  gets its device/queue from eframe's `wgpu_render_state` and must stay ABI/API
  paired with `egui-wgpu`.
- Kept a direct `wgpu` manifest entry anyway, but only as a feature selector:
  `eframe`'s `wgpu_no_default_features` path intentionally does not enable any
  native backend. Without TyphooN selecting at least one backend, release startup
  panics with `No wgpu backend feature that is implemented for the target
  platform was enabled`.
- The direct `wgpu` entry must match the `egui-wgpu` major pinned by eframe and
  should stay minimal (`std`, `parking_lot`, Linux `vulkan`, `wgsl`) rather than
  using upstream `wgpu/default`.

Desktop-only Android-surface follow-up:

- Removed TyphooN's direct `chrono` clock/local-timezone requirement and the
  `keyring-core` sample test-store feature. Runtime scheduling, log labels, and
  generated filenames now use UTC timestamps; keyring tests use the built-in
  mock store. This removes TyphooN's direct chrono/keyring path to
  `android_system_properties`.
- Remaining Android-named lockfile packages are upstream transitive target
  support from `winit`/`eframe` (`android-activity`, `ndk`, `jni`),
  `webbrowser` via `egui-winit` links, and `rustls-platform-verifier` via
  `reqwest`'s secure platform verifier. They are absent from the current Linux
  dependency tree except for wgpu's cross-platform Linux support crates
  (`wgpu-core-deps-windows-linux-android`, `wgpu-hal`, and their target support
  such as `android_system_properties`). Do not fork these upstream crates or use
  reqwest private `__rustls*` features only to scrub lockfile names.

## Follow-up alignment (2026-07-07)

Security-first refresh with the same governing rules: latest compatible lockfile,
minimal direct feature declarations, no avoidable direct-version drift, and no
dual-major dependency trees just to chase a crates.io headline release.

- Lockfile-compatible refreshes: `crossbeam-utils` 0.8.21 → 0.8.22,
  `rustversion` 1.0.22 → 1.0.23, `tendril` 0.5.0 → 0.5.1, and
  `zerocopy`/`zerocopy-derive` 0.8.52 → 0.8.53. The `tendril` refresh removed
  `utf-8` 0.7.6 from the resolved tree.
- Direct manifest alignment: moved repeated `egui`, `futures-util`, and
  `keyring-core` declarations into `[workspace.dependencies]`. Member crates now
  select only the features they actually use (`egui/default_fonts` in the native
  binary, `futures-util/std+sink` in the engine WebSocket lanes,
  `futures-util/alloc` in broker-runtime fan-outs, and no `keyring-core` sample
  feature in the shipped dependency set).
- `cargo update --workspace --dry-run --verbose` now reports only the two
  intentional non-upgrades: `generic-array` 0.14.7 and `wgpu` 29.0.4.
- `wgpu` 30 remains intentionally blocked by the `egui-wgpu` 0.35.0 pairing
  rule. TyphooN still selects `wgpu` 29 directly only as the minimal native
  backend feature selector for eframe's `wgpu_no_default_features` path.
- `generic-array` 0.14.7 remains pinned by upstream old RustCrypto lines under
  `dbus-secret-service`/`tungstenite`; this is not fixable from TyphooN direct
  manifests without replacing those upstream crates.

Validation for this pass: `cargo check --workspace`, `cargo audit`, manifest
drift scan, duplicate tree inspection, and `git diff --check` all pass. Full
workspace tests were run after the manifest and lockfile changes.

## Follow-up alignment (2026-07-08)

Security-first upstream refresh with the same policy: latest compatible lockfile,
no avoidable direct-version drift, no feature-surface widening, and no dual-major
GPU tree just to chase crates.io latest.

- Direct requirement bump: `wasm-encoder` 0.252 → 0.253 in
  `typhoon-transpiler`. The public codegen API used by
  `typhoon-transpiler/src/codegen.rs` (`Module`, `TypeSection`, `Function`,
  `Instruction`, etc.) still compiles cleanly.
- Targeted compatible lockfile updates after `cargo update --workspace` left
  them behind: `bytes` 1.12.0 → 1.12.1, `memchr` 2.8.2 → 2.8.3,
  `num-iter` 0.1.45 → 0.1.46, plus the `wasmparser` 0.252 → 0.253 companion
  update. The `num-iter` refresh also drops its `autocfg` dependency edge;
  several Windows-target package edges moved from `windows-sys 0.59` to 0.61.
- Manifest drift scan found no duplicate direct version requirements across the
  workspace. Repeated direct dependencies are either first-party workspace
  crates or shared third-party dependencies routed through `[workspace.dependencies]`
  (`serde`, `tokio`, `reqwest`, `chrono`, `egui`, `futures-util`, etc.).
- `cargo update --workspace --dry-run --verbose` now reports only the two known
  intentional non-upgrades: `generic-array` 0.14.7 and `wgpu` 29.0.4.
- `wgpu` 30 remains blocked by the eframe/egui-wgpu 0.35 pairing rule. A forced
  `cargo update -p wgpu --precise 30.0.0 --dry-run` fails against TyphooN's
  direct `wgpu = "^29"` requirement; changing it alone would create the exact
  dual-major/type-split GPU tree this ADR forbids.
- `generic-array` 0.14.7 remains pinned by upstream `crypto-common =0.1.7`
  (`=0.14.7`) through the old RustCrypto lines under `tungstenite` and
  `dbus-secret-service`. A forced `cargo update -p generic-array --precise
  0.14.9 --dry-run` fails at that exact upstream pin, so this is not fixable
  from TyphooN direct manifests without replacing upstream crates.

Validation for this pass: `cargo check --workspace`, `cargo audit`, manifest
drift scan, duplicate tree inspection, and `git diff --check` all pass.

## Follow-up alignment (2026-07-12)

The workspace-wide security refresh advanced `tokio-tungstenite` to 0.30 and
every compatible package Cargo could resolve, then audited feature unification
and every duplicate family. Direct requirements remain aligned: dependencies
used by multiple TyphooN crates inherit one requirement and default-feature
policy from `[workspace.dependencies]`; member manifests add only call-site
features.

The final independent manifest review enforced that rule for Tokio and
serde_json rather than merely documenting it: the workspace roots now carry no
Tokio capabilities and only serde_json `std`; engine, broker-runtime, and native
select their own runtime/I/O capabilities, while only engine selects
`raw_value` and byte-stable `preserve_order`. The review also removed a repeated
`keyring-core` dev declaration and TyphooN's unused `bytemuck/derive` feature.

Feature minimization removed twelve resolved packages overall (563 → 551):
unused tracing attribute/log/ANSI support, serial-test async support,
rusqlite's desktop-irrelevant WASM VFS, rfd's redundant direct Wayland client,
wasm-encoder's unused component model, the old WebSocket rand 0.9 line, and the
`webpki-roots 0.26` compatibility wrapper. The WebSocket stack now shares
RustCrypto 0.11 primitives where upstream permits and uses the platform trust
store already selected by reqwest.

The two update blockers are unchanged in principle:

- `wgpu 30` cannot move independently of eframe/egui-wgpu 0.35's wgpu 29 type
  pairing. A direct bump would create two incompatible GPU type universes.
- `generic-array 0.14.7` remains exact-pinned by the latest Secret Service
  backend's RustCrypto 0.10 family. `tokio-tungstenite 0.30` no longer
  contributes to this blocker.

Remaining `cargo tree -d` entries were traced to current upstream owners and are
not workspace drift: Secret Service versus modern RustCrypto, winit versus
clipboard Smithay generations, and independent GPU/SQLite/HTML/TLS support
stacks. Removing them locally would require dropping supported behavior or
forking upstream. See ADR-088's 2026-07-12 audit for the exact update and feature
inventory.

## Follow-up alignment (2026-07-17)

Security-first workspace centralization + latest compatible refresh (per the
initiative documented across ADR-031 and ADR-088).

- Bumped direct workspace pin: `tokio 1.52 → 1.53` (pulls 1.53.0 on resolution).
- Centralized ~20 additional direct-dependency version pins + their minimal
  feature sets into `[workspace.dependencies]`:
  `async-trait`, `bytemuck`, `crc32fast`, `eframe`, `egui_commonmark`,
  `egui_extras`, `egui_plot`, `image`, `mimalloc`, `pest`/`pest_derive`,
  `prometheus`, `regex`, `rfd`, `rmp-serde`, `rustls`, `scraper`,
  `tokio-tungstenite`, `wasm-encoder`, `wgpu`, `windows-sys`.
- All consuming member manifests (`typhoon-engine`, `typhoon-native`,
  `typhoon-transpiler`) now inherit the common version and declare only the
  additional per-crate features they actually exercise. No repeated `version =`
  strings for these crates anywhere in the workspace.
- Feature surface remains the previously-audited minimal sets (documented in
  call-site comments and prior ADR sections): e.g. engine-only `serde_json`
  extras, native-only `egui_extras` loaders, tokio capability splits by crate,
  rustls `aws_lc_rs` selector, etc.
- Result: single source of truth for versions, guaranteed common version for
  any crate used by >1 member, easier future "latest" sweeps, no change to
  resolved duplicate families (still only upstream-owned).
- Validation: `cargo check --workspace` clean (resolved and pulled tokio 1.53),
  `cargo tree -d --workspace` shows no new direct-version drift, `cargo update
  --workspace --dry-run --verbose` lists only the known intentional blockers
  (wgpu 29 vs 30, generic-array 0.14.7, plus some patch-level behinds).
- `cargo audit` expectations unchanged (quick-xml acceptances in
  `.cargo/audit.toml`).

This pass keeps the policy: latest stable, minimal direct features, no
workspace version splits, document blockers.
