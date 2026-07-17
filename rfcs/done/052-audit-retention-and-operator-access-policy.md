# RFC 052 — Audit Retention and Operator Access Policy

**Status.** Implemented (v0.36.0) — policy document at `docs/src/maintainer/audit-policy.md`
**Phase:** F8 / Pre-pilot hardening
**Project:** ciao.zinnias
**Date:** June 12, 2026
**Relationship:** Extends the audit system from RFC-013. Required before beta; acceptable as "operator-only, no defined limit" for a small internal pilot.
**RFC-079 reconciliation (2026-07-16):** `docs/src/maintainer/audit-policy.md`
is authoritative for the current closed action inventory, metadata allowlist,
legacy reset, operator queries, and Class A/B/C durability rules.

## 1. Summary

Audit logging is in place for admin actions, logout, calendar token events, and invite redemption. This RFC defines the policy: who reads audit events, how long they are retained, what metadata is allowed, and how the operator uses them for incident response.

## 2. Proposed policy (pilot default)

- **Visibility:** operator access only. No audit UI in the app. Audit events are read directly from D1 by the operator.
- **Retention:** indefinite for pilot (small data volume). A future RFC may add TTL-based cleanup.
- **Metadata allowlist:** action-specific typed fields only. Core entity IDs
  remain in dedicated columns and are not duplicated in metadata. No note
  bodies, invite plaintexts/HMACs, display names, session identifiers, exported
  content, or arbitrary JSON.
- **Export:** audit events not included in the community JSON export.
- **Incident response:** operator selects explicit core columns from
  `audit_log` with a bounded `LIMIT`; ordinary chronology queries do not select
  `metadata_json`.

## 3. Blocker

The original documentation blocker is discharged by the maintained audit
policy. Persistent incident delivery and hosted RFC-050 evidence remain open;
local source completion alone is not pilot approval.
