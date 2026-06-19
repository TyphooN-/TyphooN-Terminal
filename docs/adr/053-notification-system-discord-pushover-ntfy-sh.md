# ADR-053: Notification System — Discord, Pushover, ntfy.sh

**Status:** Implemented | **Date:** 2026-04-07

## Context

Price alerts and trading events need to reach the user when away from the terminal. Multiple notification providers are needed for redundancy and preference.

## Decision

Pluggable async notification providers in `typhoon-engine/src/notifications/`:
- **Discord** — webhook POST with embed formatting
- **Pushover** — push notifications to mobile devices
- **ntfy.sh** — self-hostable pub/sub notifications

All providers are fire-and-forget async tasks. Credentials stored in system keyring (never in files). Rate-limited per provider to avoid API bans.

## Consequences

- Users can receive alerts on phone even when terminal is minimized
- Multiple providers can fire simultaneously for redundancy
- Webhook URLs and tokens are sensitive — keyring-only storage
- No delivery guarantees (best-effort push)

See also: ADR-015 (Price Alerts), ADR-039 (Security by Design)
