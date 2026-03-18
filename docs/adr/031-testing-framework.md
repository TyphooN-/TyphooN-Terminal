# ADR-031: Testing Framework — Automated Smoke Tests

**Status:** Implemented
**Date:** 2026-03-18

## Context

With 187 Ctrl+K commands, 420+ functions, and 28,035 lines of JavaScript, manual testing is impractical. An automated test suite ensures no regressions as features are added.

## Test Suite: `smoke-test.cjs`

A Node.js-based static analysis test that runs without a browser or Tauri environment. It parses the source code and validates structural integrity.

### 12 Test Categories (602 assertions)

| Test | What it checks | Count |
|---|---|---|
| 1. Command Registration | Every CMD_PALETTE_COMMANDS entry has a matching function definition | 186 |
| 2. Unique Command Names | No duplicate names in the palette | 186 |
| 3. Unique Function Defs | No duplicate `cmd*` function definitions | ~183 |
| 4. Zero innerHTML | No innerHTML in code (only in comments) | 1 |
| 5. Request Dedup | All `invoke("get_bars")` go through `cachedGetBars` | 1 |
| 6. No eval() | Zero `eval()` calls in code | 1 |
| 7. Sandboxed Functions | `new Function()` restricted to Math/Date scope | 1 |
| 8. Interval Cleanup | clearInterval count >= setInterval count | 1 |
| 9. Indicator Functions | All 27 indicator calc functions defined | 27 |
| 10. Wasm Wrappers | All 5 wasmCalc* functions defined | 5 |
| 11. Safe DOM Helpers | All 10 DOM helper functions defined | 10 |
| 12. Dedup Layer | `cachedGetBars()` function exists | 1 |

### Running

```bash
cd frontend
node src/smoke-test.cjs
```

Exit code 0 = all pass. Exit code 1 = failures (listed in output).

### What It Catches

- Missing function definitions (renamed but not updated in palette)
- Duplicate command registrations (agent merge conflicts)
- innerHTML regressions (new features accidentally using innerHTML)
- Direct `invoke("get_bars")` bypassing the dedup layer
- eval() or unsandboxed new Function() introduced
- Interval leaks (setInterval without matching clearInterval)

### Limitations

- Static analysis only — doesn't execute commands
- Can't test runtime behavior (API responses, DOM rendering)
- Doesn't test Rust backend
- Regex-based — may have edge cases

### Future

- Browser-based E2E tests via Playwright/Puppeteer
- Rust unit tests for risk calculations
- Integration tests for Tauri IPC commands

## Bugs Found & Fixed

During test development:
1. **DES, NEWS, FA registered twice** — duplicate entries in CMD_PALETTE_COMMANDS (removed)
2. **calcPSAR naming** — function named `calcParabolicSAR`, test updated

## Consequences

- **Pro**: 602 automated assertions catch regressions instantly
- **Pro**: Runs in <1 second, no dependencies
- **Pro**: CI-friendly (exit code 0/1)
- **Con**: Static analysis only, no runtime testing
