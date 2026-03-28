# ADR-006: Security Model

**Status:** Implemented
**Date:** 2026-03-24

## Context

Trading terminals handle broker credentials, account data, and financial transactions. Web-based terminals inherit the entire browser attack surface (XSS, CSP bypass, extension injection). A native terminal can eliminate these classes of vulnerabilities by design.

## Decision

Eliminate WebView entirely, removing XSS, CSP, and DOM injection attack surfaces. All SQLite queries use parameterized statements to prevent SQL injection. Broker API keys and secrets are stored in the OS-native keyring (libsecret on Linux, Keychain on macOS, Credential Manager on Windows). The `zeroize` crate is used to scrub broker secrets from memory after use. XLSX import files (DARWIN statements) are parsed with the `calamine` crate in a read-only mode with no macro execution. No user-supplied strings are ever interpolated into queries or shell commands.

## Consequences

- No WebView means no XSS, no CSP configuration, no Content-Security-Policy headers to maintain
- Parameterized SQL eliminates injection regardless of symbol names or user input containing SQL metacharacters
- OS-native keyring provides hardware-backed credential storage where available
- `zeroize` ensures secrets do not persist in freed heap memory; mitigates cold-boot and core-dump leaks
- calamine parses XLSX as a zip-of-XML with no script execution; safe for untrusted broker export files
- Trade-off: OS keyring requires a running keyring daemon on Linux (GNOME Keyring / KDE Wallet)
