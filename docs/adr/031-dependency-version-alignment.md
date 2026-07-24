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

At the time of this 2026-05 follow-up, `native-tls` / `openssl` remained because
LAN sync built a native-TLS acceptor/connector for local WSS. ADR-115 later
removed LAN sync; the 2026-07-22 dependency tree contains neither `native-tls`
nor `openssl`. `openssl-probe` remains as a distinct certificate-location helper.

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
  shipped behavior, and one body preserved it at that time. ADR-134 later added
  the vendored render-independent `logic()` pump and separate `ui()` rendering.
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


## Follow-up alignment (2026-07-21)

Security-first comb-over per the initiative (ADR-031/088): update to latest
compatible upstream, centralize remaining, minimal features only, no direct
version drift.

- Bumped `wasm-encoder` 0.253 → 0.254 (typhoon-transpiler only). API surface
  used (Module, TypeSection, Function, Instruction, etc.) is compatible; no
  source edits required. Core WASM only (component-model disabled via
  default-features=false).
- Centralized `rand` and `serial_test` into `[workspace.dependencies]` (with
  their prior minimal feature sets). Engine now inherits; eliminates any future
  drift risk for these (even though single-consumer).
- Reviewed all feature declarations against call sites (grep + tree):
  - futures-util: engine ["std","sink"], broker ["alloc"] — exact usage
    (StreamExt/SinkExt for WS, join_all for fanouts).
  - rand: ["thread_rng"] for rand::random() in cache/backup path.
  - tokio, reqwest, rustls, egui_*, image, etc. splits and trims from prior
    passes re-verified; no unnecessary defaults pulled at direct level.
- `cargo update --workspace` applied (compatible patches within ranges; 0 new
  lock changes this pass as tree already at ceiling for declared).
- `cargo tree -d --workspace` — no new or direct-version duplicates introduced.
  Remaining splits are upstream-owned (aes 0.8/0.9, calloop 0.13/0.14,
  block-buffer/digest families via Secret Service vs our RustCrypto 0.11, etc.)
  as documented previously.
- `cargo check --workspace` clean. Full validation: manifests use workspace=
  everywhere for shared; only per-crate feature additions in members.
- `cargo audit` expectations unchanged (only the documented build-time
  quick-xml ignores).

Updated root comment and per-crate manifests to reflect the pass. This keeps
the single-source version + minimal surface rule for easy future refreshes.

## Follow-up alignment (2026-07-22)

Security-first lockfile refresh per the initiative (ADR-031/088):

- Targeted `cargo update -p` pulled latest compatible within ranges (serial to avoid index races):
  - async-trait 0.1.89 → 0.1.91 (pulls syn 3.x; upstream change, now documented dup alongside syn 2)
  - bytemuck 1.25.1 → 1.25.2
  - regex 1.13.0 → 1.13.1 + regex-automata 0.4.15 → 0.4.16
  - thiserror 2.0.18 → 2.0.19 + thiserror-impl
  - tokio 1.53.0 → 1.53.1
  - serde 1.0.228 → 1.0.229 + serde_core/derive
  - serde_json 1.0.150 → 1.0.151
- `cargo udeps --workspace`: "All deps seem to have been used."
- `cargo check --workspace` clean (1m04s).
- `cargo tree -d --workspace`: new syn v2/v3 dup from async-trait update; thiserror now unified on v2.0.19; all other dups are the documented upstream-owned families (aes 0.8/0.9, block-buffer/digest 0.10/0.12, calloop 0.13/0.14, smithay-client-toolkit 0.19/0.20, thiserror 1/2).
- `cargo audit`: clean (551 crates scanned; only the two build-time quick-xml RUSTSEC ignores in .cargo/audit.toml).
- `python .../cargo_manifest_drift.py`: drift_check=OK.
- No new direct version requirements or manifest feature expansions. Lockfile-only compatible security/bugfix refreshes.
- Blockers reconfirmed unchanged (cannot be fixed locally):
  - wgpu 29 (must track egui-wgpu 0.35 pin; 30 would create dual-major GPU tree)
  - generic-array 0.14.7 (exact pin from latest dbus-secret-service-keyring-store's old RustCrypto line)
- Updated root Cargo.toml comment + this ADR. Cargo.lock carries the patches.

Policy held: latest stable compatible, minimal direct features only, no workspace drift, upstream dups documented not removed locally.

## Follow-up verification comb-over (2026-07-22)

Security-first initiative verification pass after Claude-assisted centralization + refresh changes (ADR-031/088 policy):

- Verified no manifest or version changes needed: `cargo search` + declared pins confirm all at latest stable (or intentionally pinned per pairing rule); `cargo update --workspace` reports "Locking 0 packages" (tree at ceiling for declared ranges).
- No multiple versions of same crates declared: all repeated direct deps centralized in `[workspace.dependencies]`; grep shows zero repeated `version = ` strings for third-party crates in member manifests (only platform `dbus-secret-service-keyring-store = { version = "1" }` target-dep).
- Minimal features only: re-audited via call-site grep + `cargo tree --edges features`; every direct declaration uses `default-features = false` + exactly the exercised features (e.g. engine tokio rt+sync+time+macros+net+io-util + futures std+sink; native rt-multi-thread + egui_extras image+webp+http + rustls aws_lc_rs; broker futures alloc only; no unnecessary like derive on zeroize in engine, svg on egui_extras, etc.).
- `cargo check --workspace` clean (incremental 0.3s).
- `cargo udeps --workspace`: "All deps seem to have been used."
- `cargo tree -d --workspace`: 52 dup families, *all* upstream-owned (no direct TyphooN drift); same families as prior (aes 0.8/0.9 vs 0.9/0.11, block-buffer/digest 0.10/0.12, calloop 0.13/0.14 + smithay, syn 2/3 from async-trait, thiserror 1/2, generic-array 0.14.7, wgpu etc.).
- `python .../cargo_manifest_drift.py`: drift_check=OK.
- `cargo audit` expectations unchanged (clean except documented build-time quick-xml RUSTSEC-2026-0194/0195 ignores in .cargo/audit.toml).
- No regressions from recent Claude changes (centralize 20+ crates, wasm bump, feature normalizations, bare-string fixes): all verifications pass; call sites compile and use the inherited workspace versions/features without modification.
- Blockers unchanged and documented: wgpu@29 (egui-wgpu/eframe 0.35 pin; 30 would split GPU types), generic-array@0.14.7 (latest dbus-secret-service-keyring-store still on old RustCrypto line).
- Root Cargo.toml comment and per-crate rationale comments remain accurate.

Policy re-confirmed with zero edits required. Future "latest" sweeps remain one-place (workspace table) + documented non-upgrades.

## Upstream deduplication and advisory closure (2026-07-22)

A fresh security-first upstream pass did not trust `cargo update --workspace`
reporting `Locking 0 packages`: every one of its 42 reported newer packages was
probed serially with a targeted precise dry run. Thirty-eight were individually
compatible. Re-resolution then exposed four more compatible releases. The
lockfile now carries the complete compatible refresh, including:

- TLS/network/runtime: `aws-lc-rs` 1.17.1 → 1.17.3 (`aws-lc-sys` 0.42 →
  0.43), `rustls` 0.23.41 → 0.23.42, futures 0.3.32 → 0.3.33,
  `http-body` 1.0.1 → 1.1.0, `http-body-util` 0.1.3 → 0.1.4, `hyper`
  1.10.1 → 1.11.0, `mio` 1.2.1 → 1.2.2, and `socket2` 0.6.4 → 0.6.5.
- Parser/proc-macro/tooling: pest family 2.8.7 → 2.8.8, `proc-macro2`
  1.0.106 → 1.0.107, `quote` 1.0.46 → 1.0.47, `syn` 2.0.118 →
  2.0.119, `toml_edit` 0.25.12 → 0.25.13, `winnow` 1.0.3 → 1.0.4,
  and `foreign-types-macros` 0.2.3 → 0.2.4.
- Platform/data/crypto patches: `bitflags`, `cc`, `cfg_aliases`, `fastrand`,
  `libc`, `polyval`, `portable-atomic`, `self_cell`, `simd-adler32`,
  `simd_cesu8`, `time`/`time-macros`, `tokio-macros`, `uuid`, both
  webpki root packages, `zerocopy`/derive, and `zmij` moved to their latest
  compatible releases.
- Wayland moved coherently: `wayland-backend` 0.3.15 → 0.3.16,
  `wayland-client` 0.31.14 → 0.31.15, and `wayland-scanner` 0.31.10 →
  0.31.11. This advances quick-xml 0.39.4 → 0.41.0 and closes the two
  previously accepted build-time advisories (RUSTSEC-2026-0194/0195).

Resolved packages fell **551 → 550** and extra versions fell **44 → 43**:
the Wayland/platform refresh removed `windows-sys 0.59`, leaving only 0.52
(winit/ring/glutin compatibility) and current 0.61. The final graph has 507
unique names and 40 duplicate families. Every family was traced through Cargo
metadata reverse edges. None is direct TyphooN drift or locally removable
without dropping supported behavior or downgrading current security lines:

- latest Secret Service still owns RustCrypto 0.10-era aes/cipher/digest/hmac/
  sha2, while TyphooN uses current RustCrypto 0.11;
- winit 0.30 and clipboard/UI owners span Smithay/calloop, ObjC, CoreFoundation,
  rustix, and thiserror generations;
- independent GPU, HTML/mime, TLS, SQLite, and target-support owners retain
  their hashbrown/phf/getrandom/rand/bitflags/platform generations;
- syn 3 is used by current async-trait/serde/thiserror/foreign-types derives,
  while other current proc macros remain on syn 2.

Forty-four direct crate headlines were checked against crates.io. Stable direct
requirements are current; rustls 0.24 is prerelease only. The final compatible
dry run reports only the two established holds: `generic-array 0.14.7` is
exact-pinned by `crypto-common 0.1.7` under the latest Secret Service backend,
and wgpu 30 cannot move independently of eframe/egui-wgpu 0.35's wgpu 29 type
universe. Manifest drift remains clean and direct feature policy did not widen.

Validation: `cargo check --workspace --all-targets` is warning-free; the full
workspace suite passes 2,575 tests with 6 ignored; `cargo audit` is clean with
no ignores; the manifest drift scan and `git diff --check` pass.


## 2026-07-24 comb-over — one lockfile refresh, one new documented hold

Re-verified every direct requirement against crates.io and re-ran the audit
gates. Findings:

- **`rustls-pki-types` 1.15.0 → 1.15.1** (lockfile-only, targeted
  `cargo update -p rustls-pki-types`). This is the only in-range upgrade the
  tree was behind on. Deliberately targeted rather than a blanket
  `cargo update`: the blanket run also wanted to add `windows-sys` 0.59 **and**
  0.60 plus eleven `windows-*` target crates alongside the existing 0.52/0.61,
  which trades a patch bump for four concurrent Windows binding generations in
  the lockfile. The targeted update leaves the crate count unchanged at 550.
- **New hold — `base64` 0.22.1 (0.23.0 available).** `base64` is a single
  import site (`typhoon-engine/src/broker/kraken/mod.rs`, the `AddOrder`
  signer), but `hyper-util`, `reqwest`, and `ureq`/`ehttp`/`egui_extras` all
  still require 0.22. Moving TyphooN's direct requirement alone would put two
  `base64` majors in the tree for no security benefit — there is no advisory on
  0.22.1. Revisit when the HTTP stack moves. Note 0.23 also adds a
  `simd-unsafe` **default** feature; whenever this hold lifts, the existing
  `default-features = false, features = ["alloc"]` posture must be kept so the
  new unsafe SIMD path stays off.
- **Existing holds unchanged.** `wgpu` 29.0.4 (eframe/egui-wgpu 0.35 pins the
  wgpu 29 type universe; 30 cannot move independently) and `generic-array`
  0.14.7 (exact-pinned by `crypto-common 0.1.7` under the latest
  `dbus-secret-service-keyring-store`). `rustls` 0.24 remains prerelease
  (`0.24.0-dev.1`) — not a candidate.
- **Duplicate families re-traced, no direct drift.** Every duplicate resolves
  to an upstream owner: the RustCrypto 0.10-era `aes`/`cipher`/`digest`/
  `hmac`/`sha2`/`block-buffer` set comes in through
  `dbus-secret-service`, and the `calloop`/`smithay-client-toolkit`/`rustix`/
  `thiserror` 1-vs-2 split comes in through `winit` 0.30. No TyphooN manifest
  contributes a second version of anything.

Gates: `cargo audit` clean over 550 crates with `ignore = []` (no accepted
advisories — the 2026-07-22 `quick-xml` 0.41 move closed
RUSTSEC-2026-0194/0195 and nothing has re-opened); `cargo audit --deny warnings`
also clean, so there are no unmaintained/unsound informational advisories in the
tree; `cargo deny check advisories` and `cargo deny check bans` both pass.
