# RFC 014 — Observability, Audit, and Privacy Logging

**Status.** Implemented (v0.5.0)
**Phase:** M5 / UX and Release Hardening  
**Project:** ciao.zinnias  
**Date:** June 11, 2026  
**Reconciled:** adds a request_id correlation ID propagated across Service Bindings (the multi-worker gap).
**RFC-079 reconciliation (2026-07-16):** the original best-effort/stringly
typed design below is historical. Current required behavior is the closed,
atomic, recursively sanitized model in RFC-079 and the maintainer audit policy.
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

### Current RFC-079 boundary

- Class A mutations and their audit evidence commit in one D1 batch; no-op or
  replay transitions do not manufacture success audits.
- Class B export authorization/acknowledgement is audited before protected
  disclosure and returns generic `503` when audit construction or storage
  fails.
- `session.logout` is the only Class C exception: revocation completes first,
  the audit attempt is awaited without a session identifier, failure emits a
  bounded incident, and the cookie is still cleared.
- Actions and metadata are closed Rust enums. Arbitrary strings/JSON and direct
  audit inserts outside `workers/ssr/src/audit.rs` are forbidden.
- Structured Worker events contain request ID, canonical action/outcome or
  bounded failure category/route class only. Raw actor, community, target,
  session, metadata, SQL/bind, content, and credential material are forbidden.

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
- Errors use bounded categories rather than raw platform/SQL/debug output.
- Log level policy is documented.
- Every log line for a request carries the same `request_id`; logs persist via Logpush to R2/S3 (isolates have no filesystem — RFC-016).

---

## 9. Test Plan

- Unit tests for redaction helper.
- Integration tests verifying audit records.
- Log snapshot tests for sensitive actions.
- Manual incident simulation: non-member event access denied and logged safely.

RFC-079 source gates, local real-D1 rollback/concurrency proofs, and RFC-050
exact-candidate hosted negative evidence supersede the original generic test
language. Local passing gates do not prove persistent Logpush delivery.

---

## 10. Open Questions / Decisions

Decision: product analytics are not included in MVP observability. Operational health and safety audit are allowed.
