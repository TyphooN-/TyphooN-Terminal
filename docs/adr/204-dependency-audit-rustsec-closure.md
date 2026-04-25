# ADR-204: Dependency Audit and RustSec Advisory Closure

**Date:** 2026-04-25
**Status:** Accepted
**Related:** `Cargo.toml`, `engine/Cargo.toml`, `cli/Cargo.toml`, `mql5-compiler/Cargo.toml`, `Cargo.lock`, ADR-051 (dependency alignment), ADR-064 (performance security audit)

## Context

The workspace had not had a dependency-audit pass since ADR-051's alignment
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

Phase 1 leaves `paste` as the sole irreducible RustSec warning
(transitive via `thirtyfour` and `image → ravif → rav1e`; no
upstream alternative). Take the rest of the available majors:

- **`rand 0.9 → 0.10`**: one call site (`rand::random()` in
  `engine/src/core/lan_sync.rs`); API survived the bump.
- **`wasm-encoder 0.246 → 0.247`**: `mql5-compiler` only;
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

### Out of scope (intentionally)

- **`paste 1.0.15`** stays because `thirtyfour` and `image`'s
  AVIF chain depend on it; we do not own those crates.
- **`reqwest 0.12.x` duplicate** present alongside `0.13.2`
  through `thirtyfour 0.36`; deferred until thirtyfour ships
  a release on `reqwest 0.13`.

## Consequences

- **All RustSec vulnerabilities closed.** `cargo audit` returns 0
  vulnerabilities, 1 unfixable warning (`paste`).
- **Workspace test suite passes 1905 / 1905** lib tests across the
  five crates — no behavioral regression from any of the major
  bumps.
- **Source touch is minimal** — the only edits are two
  `use hmac::KeyInit` additions. Everything else moved through
  Cargo.toml or Cargo.lock, which is what a healthy upgrade pass
  should look like.
- **Audit cadence is now established.** Future passes can use the
  same two-phase shape: `cargo audit` for advisories,
  `cargo update` for compatible bumps, hand-picked majors for the
  rest. Recommended cadence is monthly, or whenever
  `cargo audit` flags a new vulnerability, whichever comes first.
- **Residual `paste` warning is logged here** so future audits don't
  re-discover it as new — closure is gated on upstream
  `thirtyfour` and `image` migrating away from `paste`.

## Validation

- `cargo audit` — 0 vulnerabilities, 1 warning (`paste 1.0.15`,
  unfixable transitive).
- `cargo build --workspace` — clean.
- `cargo test --workspace --lib` — 1905 passed, 0 failed.
- `Cargo.lock` re-locked with 783 crate dependencies (was 769
  pre-update).
