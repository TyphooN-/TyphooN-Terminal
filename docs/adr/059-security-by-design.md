# ADR-059: Security by Design — Credential & Data Protection

**Status:** Implemented | **Date:** 2026-03-27 | **Updated:** 2026-03-30

## Context

TyphooN Terminal handles sensitive credentials (broker API keys, passwords) and financial data. Security must be a core architectural principle, not an afterthought.

## Decisions

### 1. Credential Storage: OS-Native Keyring Only

All secrets are stored in the OS-native secure credential store:
- **Linux:** libsecret (GNOME Keyring / KDE Wallet)
- **macOS:** Keychain
- **Windows:** Credential Manager

Service name: `typhoon-terminal`

Stored credentials:
- `alpaca_api_key`, `alpaca_secret`
- `finnhub_api_key`, `fred_api_key`
- `tastytrade_username`, `tastytrade_password`

**session.json NEVER stores secrets.** It only contains non-secret configuration (indicator toggles, window positions, symbol lists, broker mode).

### 2. Credential Lifecycle

1. **Entry:** User types in Settings panel
2. **Storage:** Saved to keyring on Connect button press AND every 60 seconds (periodic sync)
3. **Loading:** Read from keyring on startup (before session.json load)
4. **Errors:** All keyring store failures logged as warnings to the UI
5. **Memory:** Engine uses `Zeroizing<String>` for API keys (zeroed on drop)
6. **CLI:** Uses `zeroize` crate for credential fields

### 3. No Plaintext Secrets Anywhere

- session.json: non-secret config only
- Log output: NEVER prints API keys, passwords, or tokens
- Error messages: strip raw HTTP error bodies (prevents token leakage)
- Network: HTTPS only (no HTTP endpoints)

### 4. Database Security

- SQLite cache: WAL mode, parameterized queries only (no SQL injection)
- Bar data: zstd-compressed binary (not human-readable)
- Integer overflow checks on binary data parsing (prevents malformed data from causing UB)
- No credentials stored in SQLite

### 5. MQL5 Compiler Security

- Sandboxed WASM execution (no host filesystem access)
- Indicators can only read bar data through imported functions
- No network access from compiled indicators

### 6. LAN Sync Message Bounds

- `MAX_KEY_LEN`, `MAX_DATA_LEN`, `MAX_WS_MESSAGE_SIZE` constants enforce strict size limits on all LAN sync WebSocket messages
- Oversized messages rejected before parsing (prevents memory exhaustion attacks)

### 7. Zero Unsafe Code

- Zero `unsafe` blocks in the entire codebase
- All GPU buffer marshalling uses `bytemuck` (Pod/Zeroable derives, `cast_slice`) instead of raw pointer casts
- Mutex-across-await fix in broker rate limiter (was holding `MutexGuard` across `.await`, now drops guard before await point)

## Consequences

- **Pro:** Credentials never touch disk in plaintext
- **Pro:** OS keyring provides hardware-backed security on supported systems
- **Pro:** Zeroized memory prevents credential leakage from memory dumps
- **Pro:** Zero `unsafe` code — no undefined behavior surface
- **Pro:** LAN sync bounded message sizes prevent resource exhaustion
- **Con:** Requires running keyring daemon on Linux (GNOME Keyring / KDE Wallet)
- **Con:** Keyring failures require re-entering credentials (logged as warnings)
