# ADR-088: Dependency Audit and RustSec Advisory Closure

**Date:** 2026-04-25
**Updated:** 2026-05-06
**Status:** Implemented
**Related:** `Cargo.toml`, `engine/Cargo.toml`, `cli/Cargo.toml`, `typhoon-transpiler/Cargo.toml`, `native/Cargo.toml`, `vendor/thirtyfour/Cargo.toml`, `Cargo.lock`, ADR-031 (dependency alignment), ADR-044 (performance security audit)

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

Phase 1 left `paste` as the sole irreducible RustSec warning at the
time, transitively via `thirtyfour` and `image → ravif → rav1e`.
Take the rest of the available majors:

- **`rand 0.9 → 0.10`**: one call site (`rand::random()` in
  `engine/src/core/lan_sync.rs`); API survived the bump.
- **`wasm-encoder 0.246 → 0.247`**: `typhoon-transpiler` only;
  no source edits.
- **RustCrypto family**: `sha2 0.10 → 0.11`, `hmac 0.12 → 0.13`,
  `pbkdf2 0.12 → 0.13` (drop unused `simple` feature). One
  trait-import change: `new_from_slice` moved from `Mac` to
  `KeyInit`, so `engine/src/broker/kraken_broker.rs` and
  `engine/src/core/lan_sync.rs` add `KeyInit` to their `use hmac::`
  lines.
- **`tokio-tungstenite 0.28 → 0.29`**: call sites already used
  `Utf8Bytes` via `.into()` from prior work, so the bump was a
  Cargo.toml-only edit.
- **`zip 7 → 8`**: `engine/src/core/darwin.rs` only; APIs
  (`ZipArchive::new`, `ZipWriter::new`, `SimpleFileOptions`)
  survived.

### Phase 3 — residual warning closure

The 2026-05-06 follow-up closed the remaining dependency warning that
was previously documented as out of scope:

- `native` now depends on `image` with `default-features = false` and
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
