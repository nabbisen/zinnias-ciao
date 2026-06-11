# RFC 019 — Retention, Soft Delete, and Data Lifecycle

**Status.** Proposed  
**Phase:** M6 / Deployment Readiness  
**Project:** ciao.zinnias  
**Date:** June 11, 2026  
**Reconciled:** attendance retained per event_day; one note per event in event_notes (Event→EventDay→Attendance grain).
**Related roadmap milestone:** M6 / Deployment Readiness  

---

## 1. Summary

This RFC defines data lifecycle decisions for cancellation, member removal, note deletion, session expiration, and offline cache. It prevents accidental data loss and supports safe audit behavior.

---

## 2. Goals

- Prefer soft deletion/cancellation for user-visible history.
- Define removed-member behavior.
- Define deleted-note behavior.
- Define session and offline cache lifecycle.
- Preserve enough auditability for admin actions.

---

## 3. Non-Goals

- No analytics retention policy.
- No user self-service full data export in MVP.
- No legal/compliance automation beyond basic privacy-aware retention.
- No hard-delete UI for normal admins.

---

## 4. External Behavior

User-visible lifecycle:

- Cancelled events remain visible as cancelled unless hidden by future policy.
- Removed members lose access.
- Deleted notes disappear from normal member view.
- Session expiration returns to Join.
- Logout clears private local data.

---

## 5. Internal Design

Lifecycle rules:

| Data | MVP lifecycle |
|---|---|
| Community | Active/inactive; operator controls deletion. |
| Membership | `removed_at` marks removal. |
| Event | scheduled/cancelled; hard delete not normal admin path. |
| Attendance (per event_day) | retained for event/day history unless policy changes. |
| Note (event_notes, per event) | `note_deleted_at`/`hidden_by_admin_at` removes it from normal view; audit without full body. |
| Invite code | expires, used, or revoked; plaintext never retained. |
| Session | finite expiry/revocation. |
| Offline cache | clear/lock on logout/session expiration. |

Data retention should be conservative and documented. If legal deletion requirements are later needed, add a dedicated RFC.

---

## 6. Data and API Design

APIs affected:

```text
POST /api/admin/events/:event_id/cancel
DELETE /api/events/:event_id/my-note
POST /api/admin/communities/:community_id/members/:membership_id/remove
POST /api/session/logout
```

Responses should tell frontend how to update local state and queued mutations.

---

## 7. Security, Privacy, and Safety

- Soft-deleted data must not remain accessible to unauthorized users.
- Audit records must not preserve sensitive content unnecessarily.
- Removed member historical notes/statuses may remain visible only according to community history policy and must not grant access.
- Offline cache clearing is required on logout/session invalidation.

---

## 8. Acceptance Criteria

- Event cancellation does not hard-delete normal event record.
- Deleted note is absent from normal event detail.
- Removed member cannot access community.
- Session expiration prevents further sync until rejoin.
- Offline cache lifecycle is implemented and tested.

---

## 9. Test Plan

- Cancellation retention tests.
- Note deletion visibility tests.
- Removed member access tests.
- Offline cache clear tests.
- Audit metadata tests.

---

## 10. Open Questions / Decisions

Open decision: how long cancelled events remain visible in Home. Recommendation: show upcoming cancelled events until their end date passes, then move to Past/history.
