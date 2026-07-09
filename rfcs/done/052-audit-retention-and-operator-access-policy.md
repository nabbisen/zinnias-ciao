# RFC 052 — Audit Retention and Operator Access Policy

**Status.** Implemented (v0.36.0) — policy document at `docs/src/maintainer/audit-policy.md`
**Phase:** F8 / Pre-pilot hardening
**Project:** ciao.zinnias
**Date:** June 12, 2026
**Relationship:** Extends the audit system from RFC-013. Required before beta; acceptable as "operator-only, no defined limit" for a small internal pilot.

## 1. Summary

Audit logging is in place for admin actions, logout, calendar token events, and invite redemption. This RFC defines the policy: who reads audit events, how long they are retained, what metadata is allowed, and how the operator uses them for incident response.

## 2. Proposed policy (pilot default)

- **Visibility:** operator access only. No audit UI in the app. Audit events are read directly from D1 by the operator.
- **Retention:** indefinite for pilot (small data volume). A future RFC may add TTL-based cleanup.
- **Metadata allowlist:** entity type, entity ID, action, timestamp. No note bodies, invite plaintexts, display names, or session tokens.
- **Export:** audit events not included in the community JSON export.
- **Incident response:** operator queries `SELECT * FROM audit_events WHERE … ORDER BY created_at DESC LIMIT 50` directly.

## 3. Blocker

No code work until retention and access policy are confirmed. Document first.
