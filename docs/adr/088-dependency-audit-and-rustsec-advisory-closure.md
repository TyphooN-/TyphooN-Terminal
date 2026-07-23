# ADR-088: Dependency Audit and RustSec Advisory Closure

**Date:** 2026-04-25
**Updated:** 2026-07-22
**Status:** Implemented
**Related:** active workspace `Cargo.toml` files, `Cargo.lock`, ADR-031 (dependency alignment), ADR-044 (performance security audit). Historical CLI/vendor manifests referenced by the original audit are no longer on active master.

## Context

The workspace had not had a dependency-audit pass since ADR-031's alignment
work. Three live RustSec advisories were open against the resolved
dependency tree:

- **RUSTSEC-2026-0098** — `rustls-webpki ≤ 0.103.11`: name constraints
  incorrectly accepted for URI names.
- **RUSTSEC-2026-0099** — same crate / version: name constraints
  incorrectly accepted for wildcard certs.
- **RUSTSEC-2026-0104** — same crate: reachable panic in CRL parsing.

All three reach the workspace transitively through `reqwest`,
`axum-server`, and `hyper-rustls`.

Five RustSec warnings were also open:

- `core2 0.4.0` — yanked, unmaintained (RUSTSEC-2026-0105), via
  `bitstream-io` → `rav1e` → `image`.
- `paste 1.0.15` — unmaintained (RUSTSEC-2024-0436), via `thirtyfour`
  and `rav1e`.
- `rand 0.8.5` and `rand 0.9.2` — unsound under custom-logger usage
  (RUSTSEC-2026-0097), via `phf_generator` (ratatui chain) and as a
  direct engine dep.

Beyond advisories, several direct dependencies had drifted:
`tokio 1.51 → 1.52`, `axum 0.8.8 → 0.8.9`, `clap 4.6.0 → 4.6.1`,
`mimalloc 0.1.48 → 0.1.50`, `rustls 0.23.37 → 0.23.39`,
`wasm-bindgen 0.2.117 → 0.2.118`, plus 50+ transitive bumps. And
several direct deps had semver-major releases waiting:
`rand 0.10`, `wasm-encoder 0.247`, `sha2 0.11` / `hmac 0.13` /
`pbkdf2 0.13` (RustCrypto family bump),
`tokio-tungstenite 0.29`, `zip 8`.

## Decision

Run a two-phase audit pass, gated on the workspace test suite passing
between phases.

### Phase 1 — `cargo update`

Apply only compatible-range updates. Result:

- `rustls-webpki 0.103.10 → 0.103.13` — closes
  RUSTSEC-2026-0098/0099/0104 inside the existing 0.103.x range.
- `bitstream-io 4.9.0 → 4.10.0` — drops yanked `core2`
  (replaced by `no_std_io2`), closing RUSTSEC-2026-0105.
- `rand 0.8.5 → 0.8.6` and `rand 0.9.2 → 0.9.4` — closes
  RUSTSEC-2026-0097 in both major lines.
- 50+ other transitive bumps including
  `tokio 1.51.1 → 1.52.1`, `axum 0.8.8 → 0.8.9`, `rustls
  0.23.37 → 0.23.39`, `wasm-bindgen 0.2.117 → 0.2.118`, `clap
  4.6.0 → 4.6.1`, `mimalloc 0.1.48 → 0.1.50`.

No source edits required. `cargo build --workspace` and
`cargo test --workspace --lib` clean post-update.

### Phase 2 — semver-major bumps

The paths in this original phase are historical. LAN sync, Darwin, the old CLI,
and their source files were removed from active `master` in June 2026; the current
dependency state is recorded by the July follow-up sections below.

Phase 1 left `paste` as the sole irreducible RustSec warning at the
time, transitively via `thirtyfour` and `image → ravif → rav1e`.
Take the rest of the available majors:

- **`rand 0.9 → 0.10`**: one call site (`rand::random()` in
  `typhoon-engine/src/core/lan_sync.rs`); API survived the bump.
- **`wasm-encoder 0.246 → 0.247`**: `typhoon-transpiler` only;
  no source edits.
- **RustCrypto family**: `sha2 0.10 → 0.11`, `hmac 0.12 → 0.13`,
  `pbkdf2 0.12 → 0.13` (drop unused `simple` feature). One
  trait-import change: `new_from_slice` moved from `Mac` to
  `KeyInit`, so `typhoon-engine/src/broker/kraken_broker.rs` and
  `typhoon-engine/src/core/lan_sync.rs` add `KeyInit` to their `use hmac::`
  lines.
- **`tokio-tungstenite 0.28 → 0.29`**: call sites already used
  `Utf8Bytes` via `.into()` from prior work, so the bump was a
  Cargo.toml-only edit.
- **`zip 7 → 8`**: `typhoon-engine/src/core/darwin.rs` only; APIs
  (`ZipArchive::new`, `ZipWriter::new`, `SimpleFileOptions`)
  survived.

### Phase 3 — residual warning closure

The 2026-05-06 follow-up closed the remaining dependency warning that
was previously documented as out of scope:

- `typhoon-native` now depends on `image` with `default-features = false` and
  `features = ["webp"]`, matching the actual screenshot-export need
  and dropping the unused AVIF/ravif/rav1e dependency chain.
- The vendored `thirtyfour 0.36.1` manifest keeps the upstream crate
  name but resolves its `paste` dependency to maintained `pastey`
  (`package = "pastey"`), so no source call sites need to change.
- The local `thirtyfour` patch already resolves to `reqwest 0.13.2`;
  the former `reqwest 0.12.x` duplicate is no longer present.

### Out of scope (intentionally)

No dependency-audit items are intentionally deferred after Phase 3.

## Consequences

- **All RustSec vulnerabilities and known warnings closed.** The
  resolved tree no longer contains `paste`, `rav1e`, or a duplicate
  `reqwest 0.12.x`.
- **Workspace lib suite passes 1932 / 1932 non-ignored tests**
  with 3 ignored tests — no behavioral regression from any of the
  major bumps.
- **Source touch is minimal** — the original pass only needed two
  `use hmac::KeyInit` additions. Phase 3 is manifest-only.
- **Audit cadence is now established.** Future passes can use the
  same two-phase shape: `cargo audit` for advisories,
  `cargo update` for compatible bumps, hand-picked majors for the
  rest. Recommended cadence is monthly, or whenever
  `cargo audit` flags a new vulnerability, whichever comes first.
- **Screenshot export scope is explicit.** Native screenshot export
  remains lossless WebP, but the crate no longer enables unrelated
  image formats just to get WebP encoding.

## Validation

- `cargo tree -i paste` — no matching packages.
- `cargo tree -i rav1e` — no matching packages.
- `cargo tree -i reqwest` — single `reqwest 0.13.2` instance.
- `cargo build --workspace` — clean.
- `cargo test --workspace --lib` — 1932 passed, 0 failed, 3 ignored.
- `Cargo.lock` re-locked with 740 crate dependencies after Phase 3
  (was 783 after the original audit pass).

## Follow-up audit (2026-07-02)

Monthly-cadence pass per the shape above. `cargo audit` opened with three
vulnerabilities and two unsound warnings against the resolved tree:

- **RUSTSEC-2026-0185** (HIGH) — `quinn-proto 0.11.14`: remote memory
  exhaustion via unbounded out-of-order stream reassembly; reachable through
  `reqwest → quinn`. Closed by `cargo update` → 0.11.15.
- **RUSTSEC-2026-0186** (unsound) — `memmap2 0.9.10` unchecked pointer
  offset, via winit/smithay. Closed by `cargo update` → 0.9.11.
- **RUSTSEC-2026-0190** (unsound) — `anyhow 1.0.102` `Error::downcast_mut`.
  Left the tree entirely: its only path was wit-bindgen tooling that
  deduplication removed (lockfile 615 → 600 crates).
- **RUSTSEC-2026-0194 / RUSTSEC-2026-0195** (HIGH) — `quick-xml 0.39.4`
  quadratic duplicate-attribute scan and unbounded `NsReader` namespace
  allocation. **Accepted, not fixed**: the only consumer is
  `wayland-scanner 0.31.10`, a build-time code generator parsing the vendored
  Wayland protocol XML inside `wayland-protocols` crates; quick-xml is never
  linked into the runtime binary. The fix landed in quick-xml 0.41, but
  wayland-scanner (latest published) still pins `^0.39`, so no
  semver-compatible closure exists. Recorded in `.cargo/audit.toml` with the
  full rationale so `cargo audit` runs stay clean-by-default and every
  acceptance is documented in-repo. Revisit on the next wayland-rs release
  (the egui/winit stack bumps are the natural trigger).

New practice this pass establishes: **advisory acceptances live in
`.cargo/audit.toml`, each with why-safe and when-to-revisit comments.** An
un-annotated ignore is a policy violation.

Validation: `cargo audit` exits clean; full workspace suite 2403 passed /
0 failed (test count grew from 1932 with the research/test expansion since
the original pass). Semver-major closures from the same session are recorded
in ADR-031's 2026-07-02 alignment section.

## Follow-up audit (2026-07-07)

Monthly-cadence security refresh per ADR-031. `cargo audit` exits clean with
only the documented quick-xml advisory acceptances in `.cargo/audit.toml`; no
new RustSec advisories were introduced by the lockfile refresh.

Compatible lockfile updates were applied for `crossbeam-utils`, `rustversion`,
`tendril`, and `zerocopy`/`zerocopy-derive`; the `tendril` update removed the
old `utf-8` crate from the resolved tree. The remaining update blockers are
not RustSec findings: `wgpu` 30 is held by the `egui-wgpu` 0.35 pairing rule,
and `generic-array` 0.14.7 is an upstream old-RustCrypto transitive hold under
Secret Service / WebSocket dependencies.

## Follow-up audit (2026-07-08)

Security-first upstream crate comb-over per ADR-031. `cargo audit` exits clean
with only the documented quick-xml advisory acceptances in `.cargo/audit.toml`;
no new RustSec advisories were introduced by the lockfile refresh.

Updates applied:

- Direct: `wasm-encoder` 0.252 → 0.253 in `typhoon-transpiler`.
- Lockfile-compatible: `bytes` 1.12.0 → 1.12.1, `memchr` 2.8.2 → 2.8.3,
  `num-iter` 0.1.45 → 0.1.46, `wasmparser` 0.252 → 0.253. The `num-iter`
  update removes its `autocfg` edge; several Windows-target dependency edges
  moved off `windows-sys 0.59` to 0.61.

Post-refresh `cargo update --workspace --dry-run --verbose` reports only the
same intentional non-upgrades: `generic-array` 0.14.7 and `wgpu` 29.0.4.
`cargo upgrade --workspace --root-deps-only` similarly reports only the direct
`wgpu` 30 headline, which remains blocked by the eframe/egui-wgpu 0.35 pairing
rule. Forced dry-run probes confirm the blockers: `generic-array 0.14.9` is
rejected by upstream `crypto-common =0.1.7`'s exact `=0.14.7` pin, and `wgpu`
30 conflicts with TyphooN's intentional `wgpu = "^29"` direct selector for the
egui-wgpu 0.35 stack.

Validation: `cargo check --workspace`, `cargo audit`, duplicate tree inspection,
manifest drift scan, and `git diff --check` all pass.

## Follow-up audit (2026-07-12)

Full security-first manifest, feature, lockfile, advisory, and duplicate-version
comb-over across all six workspace packages.

### Upstream refresh

- Direct semver-major: `tokio-tungstenite 0.29 → 0.30`; this also moves
  `tungstenite 0.29 → 0.30` and `sha1 0.10 → 0.11`, consolidating the WebSocket
  lane onto the same modern `digest 0.11`/`cpufeatures 0.3` family as TyphooN's
  direct RustCrypto stack.
- Compatible lockfile refreshes: `bytemuck 1.25.0 → 1.25.1`,
  `bytemuck-derive 1.10.2 → 1.11.0`, `cc 1.2.66 → 1.2.67`,
  `polyval 0.7.1 → 0.7.2`, `rand 0.8.6 → 0.8.7`,
  `regex 1.12.4 → 1.13.0`, `regex-automata 0.4.14 → 0.4.15`,
  `thread_local 1.1.9 → 1.1.10`, `tinyvec 1.11.0 → 1.12.0`,
  `uuid 1.23.4 → 1.23.5`, `zerocopy`/`zerocopy-derive 0.8.53 → 0.8.54`,
  and `zmij 1.0.21 → 1.0.22`.
- `cargo update --workspace --dry-run --verbose` now reports only two
  intentional blockers: `generic-array 0.14.7` and `wgpu 29.0.4`.

### Feature and resolved-tree reduction

- Disabled unused `tracing/attributes` and `tracing-subscriber` defaults;
  retained only `tracing/std` plus subscriber `fmt` and `env-filter`. This
  removes `tracing-attributes`, `tracing-log`, and `nu-ansi-term`.
- Disabled `serial_test`'s unused async/logging defaults; the engine uses only
  synchronous `#[serial]` tests. This removes `futures-executor` from the tree.
- Disabled `rusqlite` defaults and retained only `bundled` + `cache`, both used
  by the engine. This removes desktop-irrelevant `sqlite-wasm-rs` and
  `rsqlite-vfs`.
- Disabled `wasm-encoder/component-model`; TyphooN emits core WebAssembly
  modules only. Retained `std`.
- Disabled `rfd`'s direct Wayland backend and retained the XDG desktop portal;
  TyphooN uses synchronous dialogs without a parent window handle. The app's
  actual Wayland/X11 window support remains owned by eframe/winit.
- Switched `tokio-tungstenite` from the `webpki-roots 0.26` compatibility
  wrapper to native platform roots already present for reqwest/rustls. This
  removes the duplicate `webpki-roots` major while preserving authenticated TLS.
- Made minimal feature intent explicit for shared `serde`, `serde_json`,
  `thiserror`, `zeroize`, `sha2`, `pbkdf2`, `base64`, `rusqlite`, plus engine
  `crc32fast`, `rand`, and transpiler `pest`/`pest_derive`.
- A final independent manifest review moved Tokio's capabilities out of the
  workspace root and onto the members that call them: engine gets
  `rt/sync/time/macros/net/io-util`, broker-runtime gets `rt/fs/sync/time`, and
  native owns `rt-multi-thread` plus its metrics-server features. Likewise,
  serde_json `raw_value` and byte-stable `preserve_order` are engine-only; a
  cache round-trip regression test proves the latter is load-bearing.
- Removed the redundant engine `keyring-core` dev declaration (the normal
  dependency already serves unit tests) and TyphooN's unused `bytemuck/derive`
  selector. `bytemuck_derive` remains upstream-owned by the egui/wgpu graph, so
  this correctly narrows TyphooN's edge without pretending the resolved package
  can disappear.

The resolved lockfile shrank from **563 to 551 packages**. `cargo audit` exits
clean with only the two documented build-time quick-xml acceptances in
`.cargo/audit.toml`.

### Remaining duplicate families

`cargo tree -d --workspace` contains no direct workspace version drift and no
locally avoidable duplicate identified by this pass. Remaining splits are owned
by current upstream ecosystems:

- Secret Service uses RustCrypto 0.10-era `aes/cipher/digest/hmac/sha2`, while
  TyphooN encryption uses the current 0.11-era family. The latest published
  `dbus-secret-service-keyring-store` still owns the old line.
- eframe/winit/clipboard span `calloop`/Smithay 0.13/0.14 and 0.19/0.20;
  disabling clipboard or a supported display backend would remove behavior, not
  dead surface.
- wgpu, rusqlite, scraper/HTML, TLS, and target-support owners require their
  respective `foldhash`/`hashbrown`, `phf`, `rustix`, `getrandom`, `thiserror`,
  and related semver lines. TyphooN does not declare competing direct versions.

Do not patch or fork these merely to force a cosmetically single-version tree;
retest them when their owning upstream stack releases a unifying version.

## Follow-up audit (2026-07-17)

Security-first comb-over: centralized remaining direct dep pins into
[workspace.dependencies] for version unification + minimal features.

- Updated workspace root to declare latest versions (tokio 1.53, plus 20
  others now centralized: async-trait through wgpu/windows-sys with
  default-features=false and only the features actually exercised by members).
- Refactored typhoon-engine, typhoon-native, typhoon-transpiler manifests to
  inherit from workspace (no local version strings for centralized crates).
- This eliminates any risk of future drift on these crates and makes "update
  to latest" a one-place operation.
- cargo check --workspace succeeded; lockfile updated with tokio 1.53.0.
- No new RustSec issues; duplicates unchanged (upstream only, as before).
- Feature minimality preserved from prior audits and explicit in the
  centralized declarations + per-use overrides.

See ADR-031 for the parallel entry. All direct TyphooN deps now route
through the single workspace table for common versions.


## Follow-up audit (2026-07-21)

Security-first workspace comb-over (per ADR-031/088):

- Bumped direct `wasm-encoder` 0.253 → 0.254 in workspace table; transpiler
  continues to build and function with no code changes.
- Centralized `rand = { version = "0.10", ... }` and `serial_test` (dev) into
  `[workspace.dependencies]` so all (future) consumers share one pin.
- Re-audited feature surfaces for all centralized crates against live call
  sites; no trims or expansions this pass (prior minimal sets hold: see
  engine/broker/native manifests and comments).
- `cargo check --workspace` and `cargo tree -d --workspace` clean; no direct
  version splits, no new RustSec (audit still only has the two documented
  build-time quick-xml ignores).
- `cargo update --workspace` confirmed at ceiling for declared ranges.
- Root Cargo.toml and member manifests now have zero repeated version strings
  for shared crates; rand/serial_test unified.

This pass reinforces: latest within policy, one version source, minimal
features only, duplicates only from upstream (documented).

See ADR-031 for the companion entry.

## Follow-up audit (2026-07-22)

Security-first lockfile refresh (per ADR-031/088):

- Serial targeted updates pulled:
  async-trait 0.1.89→0.1.91, bytemuck 1.25.1→1.25.2, regex+automata to 1.13.1/0.4.16, thiserror to 2.0.19, tokio to 1.53.1, serde family to 1.0.229/1.0.151.
- Introduced syn v3.0.3 (via async-trait); now two syn lines (v2 from other derives, v3 from async-trait). Documented as upstream; build and udeps clean.
- Verification identical to companion ADR-031 entry: udeps "all used", check clean, tree dups only upstream+new syn, audit clean, drift OK.
- Lockfile refreshed; manifests unchanged (already centralized/minimal in prior pass).
- Reconfirmed no avoidable direct multi-versions, features remain the audited minimal sets.

See ADR-031 for full command list and blocker details.

## Follow-up verification comb-over (2026-07-22)

Verification pass post centralization/refresh (companion to ADR-031):

- No new updates or manifest edits required (cargo update locking 0; declared at latest within policy).
- `cargo udeps --workspace`: All deps used.
- `cargo check --workspace` clean.
- `cargo tree -d`: only upstream dups, no direct multi-versions anywhere in workspace manifests.
- Drift script OK; features remain the minimal audited sets (no widening).
- `cargo audit` expectations: clean aside from the two documented build-time quick-xml ignores.
- Confirmed no regressions from prior Claude work on centralization, feature localization, version bumps.
- All crates sharing common version via workspace table; only compiling needed flags.

See ADR-031 for detailed command output and policy restatement.

## Advisory closure and upstream deduplication (2026-07-22)

The upstream re-comb closed the last accepted advisories rather than renewing
their exception. `wayland-scanner` 0.31.11 now depends on quick-xml 0.41, which
fixes RUSTSEC-2026-0194 and RUSTSEC-2026-0195. `.cargo/audit.toml` therefore has
an empty ignore list, and `cargo audit` scans all 550 resolved packages with no
vulnerability, warning, or accepted finding.

Serial precise probes found and applied 42 compatible package refreshes that
the initial workspace-wide dry run left unchanged. Besides current security and
bug-fix levels across TLS, HTTP, async runtime, parser, crypto, and platform
support, the coherent Wayland refresh removed the `windows-sys 0.59` line.
Resolved package count fell 551 → 550; 507 names remain, with 40 upstream-owned
duplicate families / 43 extra versions. Metadata reverse-edge tracing found no
direct workspace split and no safe local unification missed by this pass.

The only remaining update holds are non-advisory ecosystem constraints:
`generic-array 0.14.7` is exact-pinned by the latest Secret Service backend's
old RustCrypto line, and wgpu 29 must remain paired with eframe/egui-wgpu 0.35.
No direct feature was widened and the manifest drift check remains clean. See
the companion ADR-031 entry for the complete update and duplicate-owner groups.

Validation: warning-free all-target workspace check, 2,575 passed / 0 failed /
6 ignored, clean unignored `cargo audit`, clean manifest drift scan, and clean
diff validation.

