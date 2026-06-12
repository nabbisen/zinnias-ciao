# RFC 014 — Observability, Audit, and Privacy Logging

**Status.** Implemented (v0.5.0)
**Phase:** M5 / UX and Release Hardening  
**Project:** ciao.zinnias  
**Date:** June 11, 2026  
**Reconciled:** adds a request_id correlation ID propagated across Service Bindings (the multi-worker gap).
**Related roadmap milestone:** M5 / UX and Release Hardening  

---

## 1. Summary

This RFC defines logs, audit records, and operational observability. The system needs enough visibility to debug and investigate incidents without leaking private community data.

---

## 2. Goals

- Log operational health and errors.
- Audit security-sensitive and admin actions.
- Redact secrets and private user content.
- Support incident review for community-isolation failures.
- Keep logging simple for MVP.

---

## 3. Non-Goals

- No analytics dashboard.
- No user behavior tracking product metrics.
- No storing full note bodies in logs.
- No third-party tracking scripts.

---

## 4. External Behavior

Users do not see logs. Admins may see simple admin history only if implemented, but operator/security audit can exist separately.

User-facing messages remain plain and do not expose trace IDs unless project decides support workflow needs them.

---

## 5. Internal Design

Audit events:

- invite code generated;
- invite redeemed;
- member removed;
- event created;
- event edited;
- event cancelled;
- admin attendance override;
- admin note deletion;
- session revoked for incident response.

Operational logs:

- `request_id` (generated at the edge, propagated across Service Bindings so a request can be traced across workers);
- request route category;
- result status;
- latency;
- external error code;
- redacted actor/community IDs if needed.

Do not log:

- session secrets;
- invite plaintext;
- invite hashes;
- note body;
- full event description unless explicitly sanitized.

---

## 6. Data and API Design

Audit table:

```sql
audit_log (
  id TEXT PRIMARY KEY,
  community_id TEXT,
  actor_membership_id TEXT,
  target_kind TEXT NOT NULL,
  target_id TEXT,
  action TEXT NOT NULL,
  metadata_json TEXT,
  created_at TEXT NOT NULL
);
```

Metadata must be structured and redacted. For status override, metadata may include previous/new status but not private note content.

---

## 7. Security, Privacy, and Safety

- Logs are part of the privacy boundary.
- Audit records must not become a hidden copy of deleted notes.
- Community isolation failures should be high-severity logs.
- Rate-limit events for invite attempts should be observable without storing attempted plaintext codes.

---

## 8. Acceptance Criteria

- Admin/security-sensitive actions create audit records.
- Production logs do not contain invite codes or session values.
- Note bodies are absent from logs.
- Errors include external code and enough context for debugging.
- Log level policy is documented.
- Every log line for a request carries the same `request_id`; logs persist via Logpush to R2/S3 (isolates have no filesystem — RFC-016).

---

## 9. Test Plan

- Unit tests for redaction helper.
- Integration tests verifying audit records.
- Log snapshot tests for sensitive actions.
- Manual incident simulation: non-member event access denied and logged safely.

---

## 10. Open Questions / Decisions

Decision: product analytics are not included in MVP observability. Operational health and safety audit are allowed.
