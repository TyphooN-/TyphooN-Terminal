# ADR-006: Security Hardening

**Status:** Implemented (Pass 1)
**Date:** 2026-03-15
**Context:** Security audit identified 4 critical, 3 high, and 3 medium issues.

## Changes Made

### Critical → Fixed

1. **CSP enabled**: `"csp": "default-src 'self'; script-src 'self'; style-src 'self' 'unsafe-inline'"` — prevents external script injection
2. **Devtools debug-only**: Moved to `[features] devtools = ["tauri/devtools"]` — not included in release builds. Enable with `cargo build --features devtools`

### High → Fixed

3. **XSS in autocomplete**: Replaced `innerHTML` with `createElement` + `textContent` for symbol name display
4. **XSS in log panel**: Replaced `innerHTML` with `createElement` + `textContent`
5. **XSS in tooltip**: Changed to `textContent` with `pre-line` whitespace
6. **Input validation on orders**: Symbol validated (alphanumeric + / + ., max 20 chars), qty validated (0 < qty ≤ 1M, finite), side validated (buy/sell only)

### Accepted Risks

7. **Credentials in localStorage**: Accepted for now — Tauri webview localStorage is sandboxed per application (not shared with browsers). OS keychain integration is a future improvement. Users can uncheck "Save credentials" to avoid persisting
8. **withGlobalTauri: true**: Required for Tauri invoke() to work. The CSP prevents external scripts from exploiting this
9. **API keys in Rust memory**: Low risk — process memory access requires root/admin. `zeroize` crate integration is a future improvement

### Low → Documented

10. **Error messages expose API structure**: Acceptable for a trading tool — users need diagnostic info
11. **Symbol/trade logging**: Debug-only via `RUST_LOG` env var

## Remaining Work (Future Passes)

- OS keychain for credential storage (via `tauri-plugin-store` or system keyring)
- `zeroize` crate for API key memory cleanup
- Rate limiting on frontend (prevent rapid-fire order clicks)
- `cargo audit` and `npm audit` in CI
- Restrict Tauri command allowlist per window
