# ADR-010: Multi-Account Credential Management

**Status:** Implemented
**Date:** 2026-03-15
**Context:** Users may have multiple Alpaca accounts (paper, live, different strategies).

## Decision

Store named accounts in Tauri webview's localStorage with save/load/delete capability. Support both paper and live account types.

## Features

- **Named accounts**: Save credentials with a descriptive name (e.g., "Paper Main")
- **Account type**: Paper or Live selector — live shows red warning
- **Save checkbox**: User explicitly opts in to credential persistence
- **Account dropdown**: Select from saved accounts, auto-fills form
- **Delete button**: Remove saved accounts
- **Auto-fill**: If only one saved account, pre-fills on startup

## Security Notes (see ADR-006)

- Credentials stored in Tauri webview localStorage (sandboxed per application)
- Not accessible by browsers or other apps
- CSP prevents external script access
- Users can uncheck "Save credentials" to avoid persistence
- Future: OS keychain integration for stronger protection
