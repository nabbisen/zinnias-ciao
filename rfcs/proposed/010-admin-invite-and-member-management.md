# RFC 010 — Admin Invite and Member Management

**Status.** Proposed  
**Phase:** M4 / Admin MVP Flow  
**Project:** ciao.zinnias  
**Date:** June 11, 2026  
**Reconciled:** AD-3 (peppered HMAC invite codes), AD-4 (admin forms carry a form token); offline-write line removed.
**Related roadmap milestone:** M4 / Admin MVP Flow  

---

## 1. Summary

This RFC defines admin invite-code generation and member management. These flows must be safe because they control the community boundary.

---

## 2. Goals

- Allow admins to generate one-time invite codes.
- Show invite code only at creation time.
- Allow admins to view active members.
- Allow admins to remove members with confirmation.
- Keep used invite codes hidden from normal users.

---

## 3. Non-Goals

- No bulk invite import.
- No email/SMS sending in MVP.
- No public join links.
- No self-service account recovery.
- No granular permissions beyond admin/member.

---

## 4. External Behavior

Admin Invite screen:

```text
Create invite code
[Create code]

Code: X7Y9Z2
Share this with one person. It expires in 24 hours.
```

The code is shown only immediately after generation. Later views may show counts such as active/used/expired, not the plaintext code.

Member List screen shows display name, role, joined date, and remove action where allowed.

---

## 5. Internal Design

Invite generation:

- generate cryptographically random code;
- normalize for display/input;
- store only HMAC-SHA256(pepper, normalized_code) (AD-3); never plaintext;
- set expiration;
- bind to community and creating admin membership;
- audit generation event without storing plaintext code in audit.

Member removal:

- set `removed_at`;
- do not delete historical records;
- invalidate or restrict access on next session/API validation;
- the next request after removal is denied server-side (no offline write path, AD-1).

Admin role transfer/bootstrap:

- initial admin bootstrap may be operator-controlled or seed migration;
- removing the last admin should be blocked unless operator override exists.

---

## 6. Data and API Design

Endpoints:

```text
POST /api/admin/communities/:community_id/invite-codes
GET  /api/admin/communities/:community_id/members
POST /api/admin/communities/:community_id/members/:membership_id/remove
```

Invite response includes plaintext code only once:

```json
{"code":"X7Y9Z2","expires_at":"..."}
```

---

## 7. Security, Privacy, and Safety

- Plaintext invite code must not be stored or logged.
- Generic join errors prevent code probing.
- Admin cannot remove themselves if they are the last admin.
- Member removal is auditable.
- Removed members cannot access data after server-side validation.

---

## 8. Acceptance Criteria

- Admin can generate invite code.
- Code can be redeemed once.
- Code expires.
- Plaintext code is not visible later.
- Admin can remove member with confirmation.
- Last-admin removal is blocked.

---

## 9. Test Plan

- Invite generation entropy/format tests.
- Hash storage tests.
- Member removal access tests.
- Last-admin guard tests.
- Audit record tests.

---

## 10. Open Questions / Decisions

Open decision: exact invite code alphabet. Recommendation: avoid ambiguous characters such as O/0/I/1 if the code remains manually typed.
