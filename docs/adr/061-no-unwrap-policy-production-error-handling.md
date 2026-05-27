# ADR-061: No Unwrap Policy — Production Error Handling

**Status:** Enforced | **Date:** 2026-04-08

## Context

`.unwrap()` and `.expect()` in production code cause panics on error, crashing the entire application. In a trading terminal, a panic during market hours means lost visibility on open positions, missed alerts, and potential financial risk. Every error path must be handled gracefully.

## Decision

**Zero `.unwrap()` and `.expect()` in production code.** All fallible operations must use proper error handling:

### Allowed Patterns
- `?` operator (propagate errors to caller)
- `.unwrap_or(default)` / `.unwrap_or_default()` (safe fallback)
- `.unwrap_or_else(|e| { log_error; fallback })` (logged fallback)
- `if let Ok(v) = ... { use v }` (skip on error)
- `match` with explicit error arms
- `std::process::exit(1)` for truly fatal init errors (no async runtime, no GPU device)

### Forbidden in Production Code
- `.unwrap()` — silent panic, no context
- `.expect("message")` — panic with message, still crashes
- `panic!()` — explicit crash

### Exceptions
- **Test code** (`#[test]` functions): `.unwrap()` and `.expect()` are acceptable since test failures are expected to panic
- **Static initialization** (`OnceLock`, `Lazy`): use `.unwrap_or_else(|_| fallback)` with a safe default

### Enforcement
- Code review: reject any PR with `.unwrap()` or `.expect()` outside test modules
- `cargo clippy` with `#[deny(clippy::unwrap_used)]` can be enabled per-crate

## Consequences

- Application never panics in production — errors logged and handled gracefully
- Slightly more verbose code at error sites (worth the reliability)
- Users see error messages in the log instead of application crashes
- Trading operations continue even when non-critical subsystems fail

See also: ADR-039 (Security by Design), ADR-044 (Performance & Security Audit)
