# ADR-006: Security Model

**Status:** Implemented
**Date:** 2026-03-24

## Context

Trading terminals handle broker credentials, account data, and financial transactions. Web-based terminals inherit the entire browser attack surface (XSS, CSP bypass, extension injection). A native terminal can eliminate these classes of vulnerabilities by design.

## Decision

Eliminate WebView entirely, removing XSS, CSP, and DOM injection attack surfaces. All SQLite queries use parameterized statements to prevent SQL injection. Broker API keys and secrets are encrypted at rest with AES-256-GCM, keyed from OS keyring where available. The `zeroize` crate is used to scrub broker secrets from memory after use. XLSX import files (DARWIN statements) are parsed with the `calamine` crate in a read-only mode with no macro execution. No user-supplied strings are ever interpolated into queries or shell commands.

## Consequences

- No WebView means no XSS, no CSP configuration, no Content-Security-Policy headers to maintain
- Parameterized SQL eliminates injection regardless of symbol names or user input containing SQL metacharacters
- AES-256-GCM encryption at rest protects credentials if the config directory is copied or backed up
- `zeroize` ensures secrets do not persist in freed heap memory; mitigates cold-boot and core-dump leaks
- calamine parses XLSX as a zip-of-XML with no script execution; safe for untrusted broker export files
- Trade-off: OS keyring integration varies by platform; fallback is file-based encryption with a derived key
